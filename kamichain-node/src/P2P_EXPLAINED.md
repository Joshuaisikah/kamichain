# P2P Layer — `p2p.rs`

## Purpose

The P2P layer lets KamiChain nodes discover each other, propagate new blocks and transactions, and synchronise their chains. It uses raw TCP with newline-delimited JSON messages — no external networking library, no DHT, no gossip protocol. The design is a minimal flood network: when a node receives a block it accepts it; when it mines a block it broadcasts to all known peers.

---

## Message Protocol

All messages are a single line of JSON terminated by `\n`. The envelope uses a tagged union (`serde`'s `tag + content`):

```rust
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    NewBlock(Block),        // propagate a mined block
    GetChain,               // ask a peer for its full chain
    Chain(Vec<Block>),      // respond with the full chain
    NewTx(Transaction),     // propagate a mempool transaction
    GetPeers,               // ask for known peers
    Peers(Vec<String>),     // respond with peer addresses
}
```

On the wire these look like:
```json
{"type": "new_block", "data": { ...Block fields... }}
{"type": "get_chain"}
{"type": "chain",    "data": [ ...blocks... ]}
```

---

## `P2PLayer` Struct

```rust
pub struct P2PLayer {
    listener: TcpListener,               // accept incoming connections
    peers:    Arc<Mutex<Vec<String>>>,   // known peer addresses
    state:    Arc<RwLock<NodeState>>,
    mempool:  Arc<Mutex<Mempool>>,
}
```

`peers` is a flat list of `"host:port"` strings. There is no peer scoring, no eviction, and no maximum peer count — this is intentional simplicity for a learning/prototype node.

---

## Methods

### `new(addr, state, mempool)`

Binds a `std::net::TcpListener` synchronously (same reasoning as the RPC server — fail fast at startup). The peer list starts empty; peers are added via `connect()` or could be extended later via `GetPeers`.

### `listen_addr() -> String`

Returns the actual bound address (useful when bound to port 0 in tests).

### `peers() -> Vec<String>`

Snapshot of the current peer list.

### `listen(&self) -> anyhow::Result<!>`

Accept loop — converts the std listener to tokio, then spawns a task per incoming connection calling `handle_peer`. The spawned tasks share `Arc` clones of `state`, `mempool`, and `peers`.

### `connect(&self, peer_addr) -> anyhow::Result<()>`

Dials an outbound connection to a known peer, adds it to the peer list, and spawns `handle_peer` on the resulting stream. The spawned handler will process the first message the remote sends (or wait for one). This method is called at startup when `--peer` is provided.

### `broadcast_block(&self, block)`

Opens a fresh TCP connection to each known peer and sends a `NewBlock` message. Fires-and-forgets per peer — errors are silently swallowed because a single unreachable peer should not stop propagation to the rest.

**Why a new connection per broadcast?** Persistent connections require connection-lifecycle management (health checks, reconnection). A new connection per message is slower but keeps the code simple and stateless.

### `broadcast_tx(&self, tx)`

Same as `broadcast_block` but wraps the transaction in `NewTx`. Called when the node receives a transaction over RPC and wants to forward it to peers.

### `sync_with_peer(&self, addr) -> anyhow::Result<()>`

Bootstrap sync — used once at startup. Opens a connection to `addr`, sends `GetChain`, reads the `Chain` response, and calls `chain.replace(blocks)` to overwrite the local chain with the peer's chain.

**Security note**: this blindly trusts the peer's chain. A production node would verify the received chain (re-validate every block, check total work) before replacing the local one.

---

## `handle_peer` (private async fn)

Called for every accepted or outbound connection. Reads one line from the stream, parses it as a `Message`, and handles it:

| Message received | Action |
|------------------|--------|
| `NewBlock(block)` | Write-lock state, call `chain.add_block`. If the block is valid, also call `apply_block` to update balances. If invalid (block is behind, wrong hash), the error is swallowed — a future enhancement would send `GetChain` to catch up. |
| `GetChain` | Read-lock state, clone all blocks, send `Chain(blocks)` response. |
| `NewTx(tx)` | Lock mempool, call `mempool.add(tx)`. Invalid or duplicate transactions are silently dropped. |
| `GetPeers` | Send `Peers([])` — peer exchange is not yet implemented. |
| anything else | Ignored (`_` arm). |

**One message per connection**: Like the RPC server, each TCP connection handles exactly one message. This avoids stream-framing complexity at the cost of connection overhead.

---

## Threading Model

The P2P layer is entirely async (tokio). Shared state is accessed through the same `Arc<RwLock<NodeState>>` and `Arc<Mutex<Mempool>>` used by the miner and RPC server. Lock contention is minimised by holding locks only for the mutation itself.

```
P2P listen loop
    ├─ tokio::spawn handle_peer (connection A)
    │       └─ state.write() for ~1µs on NewBlock
    ├─ tokio::spawn handle_peer (connection B)
    │       └─ state.read() for ~1µs on GetChain
    └─ ...
```

---

## Known Limitations / Future Work

| Gap | Notes |
|-----|-------|
| No chain validation on sync | `replace()` trusts the peer blindly |
| No peer discovery | `GetPeers` returns an empty list |
| No reconnect | If a peer drops, it is never removed from the list |
| New TCP connection per broadcast | Fine for <100 peers, expensive beyond |
| No message size limit | A malicious peer could send an unbounded line |
