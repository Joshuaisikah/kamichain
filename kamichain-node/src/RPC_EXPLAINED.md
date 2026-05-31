# RPC Server — `rpc.rs`

## Purpose

The RPC server exposes the node's state over a newline-delimited JSON protocol on a TCP socket. Clients (the CLI, wallets, explorers) send a single JSON request per connection and receive a single JSON response back.

The design is deliberately minimal: one connection = one request/response pair. No persistent connections, no framing, no authentication.

---

## Wire Format

**Request** — a JSON object on a single line, terminated by `\n`:
```json
{"method": "chain_info", "params": null}
{"method": "chain_block", "params": {"index": 3}}
{"method": "tx_submit", "params": {"tx": { ...transaction fields... }}}
{"method": "wallet_balance", "params": {"address": "alice"}}
```

**Response** — a JSON object on a single line, terminated by `\n`:
```json
{"ok": true,  "result": { ... }, "error": null}
{"ok": false, "result": null,    "error": "block 99 not found"}
```

---

## Types

### `RpcRequest`

```rust
pub struct RpcRequest {
    pub method: String,
    pub params: Option<Value>,  // serde_json::Value — any JSON shape
}
```

Method names follow `category_action` naming (e.g., `chain_info`, `tx_submit`).

### `RpcResponse`

```rust
pub struct RpcResponse {
    pub ok:     bool,
    pub result: Option<Value>,
    pub error:  Option<String>,
}
```

- `RpcResponse::ok(value)` — constructs a success response.
- `RpcResponse::err(msg)` — constructs an error response.

---

## `RpcServer` Struct

```rust
pub struct RpcServer {
    listener: TcpListener,           // std listener, converted to tokio at runtime
    state:    Arc<RwLock<NodeState>>,
    mempool:  Arc<Mutex<Mempool>>,
}
```

Holds `Arc` handles to the shared state and mempool so each spawned connection handler can clone them cheaply.

### `RpcServer::new(addr, state, mempool)`

Binds a `std::net::TcpListener` synchronously (so binding errors surface at startup, not inside the async runtime). Sets `SO_NONBLOCKING` so the tokio runtime can adopt it with `TcpListener::from_std`.

### `local_port() -> u16`

Utility used in tests to discover the ephemeral port when bound to `:0`.

### `run(self) -> anyhow::Result<!>`

Converts the std listener to a tokio listener and enters an accept loop. For each incoming connection a task is spawned via `tokio::spawn` so connections are handled concurrently without blocking each other.

---

## `handle` (private async fn)

Called once per connection. Reads exactly one line from the socket, parses it as `RpcRequest`, dispatches to `dispatch`, serialises the response, and writes it back. The connection is then dropped.

**Why read only one line?** Simplicity. A blockchain node RPC doesn't need persistent sessions. Each CLI command opens a fresh TCP connection.

---

## `dispatch` (private fn)

Pattern-matches on `req.method` and delegates to the appropriate handler. All handlers hold locks for the minimum time required.

| Method | Lock held | What it does |
|--------|-----------|--------------|
| `chain_info` | `RwLock::read` | Returns chain height, latest hash, difficulty |
| `chain_block` | `RwLock::read` | Returns the serialised block at the given index |
| `tx_submit` | `Mutex::lock` | Deserialises a `Transaction`, calls `mempool.add` |
| `wallet_balance` | `RwLock::read` | Looks up address in the balance ledger |
| `node_peers` | none | Placeholder — returns empty peer list |
| anything else | none | Returns `"unknown method"` error |

### `chain_info`

```json
{"method": "chain_info"}
→ {"ok": true, "result": {"height": 5, "latest_hash": "0000abc...", "difficulty": 2}}
```

### `chain_block`

```json
{"method": "chain_block", "params": {"index": 0}}
→ {"ok": true, "result": { ...full Block JSON... }}
```

Returns an error if `index` is out of range.

### `tx_submit`

```json
{"method": "tx_submit", "params": {"tx": {"id":"...", "tx_type":"Transfer", ...}}}
→ {"ok": true, "result": {"submitted": true}}
```

The transaction is validated by `mempool.add` which checks: not coinbase, sender ≠ recipient, amount > 0, valid signature, no duplicate, mempool not full.

### `wallet_balance`

```json
{"method": "wallet_balance", "params": {"address": "alice"}}
→ {"ok": true, "result": {"balance": 150}}
```

Returns 0 for unknown addresses (not an error — the address just has no confirmed transactions yet).

---

## Error Handling

All failures return `{"ok": false, "error": "..."}` rather than closing the connection abruptly. The only hard errors are I/O failures in `handle`, which are logged to stderr via `eprintln!`.

---

## Security Notes

- No authentication. The RPC port should only be bound to `127.0.0.1` in production or protected by a firewall.
- Transaction signatures are validated in `mempool.add` via `Wallet::verify_transaction` — the RPC layer trusts the mempool's verdict.
- JSON is deserialized with `serde_json` — no `eval`, no shell injection surface.
