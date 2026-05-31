# rpc.rs — talking to the node over TCP

the RPC server is how the outside world talks to the node — CLI commands, wallets, anything that wants to query the chain or submit a transaction. I kept it extremely simple: one JSON request per TCP connection, one response back, connection closes. no persistent sessions, no framing, no auth.

---

## wire format

request — one JSON line, `\n` terminated:
```json
{"method": "chain_info"}
{"method": "chain_block", "params": {"index": 3}}
{"method": "tx_submit", "params": {"tx": { ...transaction fields... }}}
{"method": "wallet_balance", "params": {"address": "alice"}}
```

response — one JSON line back:
```json
{"ok": true,  "result": { ... }, "error": null}
{"ok": false, "result": null,    "error": "block 99 not found"}
```

`ok` is always there so the client knows immediately whether to look at `result` or `error`. simple.

---

## the types

```rust
pub struct RpcRequest {
    pub method: String,
    pub params: Option<Value>,
}
```

`params` is a raw `serde_json::Value` because each method has a different shape. I just deserialise per-method instead of trying to make a generic enum for all of them.

```rust
pub struct RpcResponse {
    pub ok:     bool,
    pub result: Option<Value>,
    pub error:  Option<String>,
}
```

`RpcResponse::ok(value)` and `RpcResponse::err(msg)` are just convenience constructors. nothing fancy.

---

## RpcServer

```rust
pub struct RpcServer {
    listener: TcpListener,
    state:    Arc<RwLock<NodeState>>,
    mempool:  Arc<Mutex<Mempool>>,
}
```

I bind the TCP listener synchronously in `new()` so if the port is taken we die immediately at startup with a clear error, not somewhere inside the async runtime. then `run()` converts it to a tokio listener and loops on `accept()`, spawning a task per connection.

`local_port()` exists just for tests — bind to `:0`, get whatever port the OS picked.

---

## one connection, one message

each accepted connection gets `handle()` which reads exactly one line, dispatches it, writes the response, and drops the connection. I went with this instead of persistent connections because:

- blockchain CLI tools fire a command and exit anyway
- framing (knowing when a message ends) is a whole problem I didn't want to solve
- stateless connections mean no cleanup on disconnect

---

## what each method does

| method | locks | what it does |
|--------|-------|-------------|
| `chain_info` | read | height, latest hash, difficulty |
| `chain_block` | read | full block JSON by index |
| `tx_submit` | read then mempool lock | balance check + add to mempool |
| `wallet_balance` | read | confirmed balance for an address |
| `node_peers` | none | returns `[]` — not implemented yet |
| anything else | none | error |

### chain_info

```json
{"method": "chain_info"}
→ {"ok": true, "result": {"height": 5, "latest_hash": "0000abc...", "difficulty": 2}}
```

### chain_block

```json
{"method": "chain_block", "params": {"index": 0}}
→ {"ok": true, "result": { ...full Block JSON... }}
```

out of range index gives a proper error, not a panic.

### tx_submit

```json
{"method": "tx_submit", "params": {"tx": {"id":"...", "tx_type":"Transfer", ...}}}
→ {"ok": true, "result": {"submitted": true}}
```

this one does two things before hitting the mempool. first reads the sender's confirmed balance from state:

```rust
let sender_balance = state.read().unwrap().balance_of(&tx.sender);
mempool.lock().unwrap().add(tx, sender_balance)
```

then `mempool.add` runs through: not coinbase, sender ≠ recipient, amount > 0, valid signature, `amount + fee <= balance`, not a duplicate, pool not full. any failure comes back as `{"ok": false, "error": "..."}`.

a fresh wallet with no on-chain balance gets rejected with `"insufficient balance"` — which is correct. you need coins before you can spend them.

### wallet_balance

```json
{"method": "wallet_balance", "params": {"address": "alice"}}
→ {"ok": true, "result": {"balance": 150}}
```

returns 0 for addresses that have never received anything. not an error.

---

## locking

I take the state read lock just long enough to get the balance (or chain info), then drop it before touching the mempool. I never hold both locks simultaneously. that's important because the miner holds the mempool lock for the entire mining duration — if the RPC server tried to grab both, we'd deadlock.

---

## a note on security

no auth, no TLS. you should only run this on localhost or behind a firewall. signature verification happens inside `mempool.add`, so the RPC layer itself doesn't need to know about keys — the mempool handles it.
