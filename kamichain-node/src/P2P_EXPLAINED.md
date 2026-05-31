# p2p.rs — talking to other nodes

the P2P layer handles everything node-to-node: broadcasting new blocks and transactions, syncing the chain when connecting to a peer. I went with raw TCP and newline-delimited JSON — no libp2p, no DHT, no gossip protocol. just open a socket, send a JSON line, optionally read one back.

---

## the message format

all messages are a single JSON line terminated by `\n`. I used `serde`'s adjacently-tagged enum so every message has a `"type"` and optional `"data"`:

```rust
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    NewBlock(Block),
    GetChain,
    Chain(Vec<Block>),
    NewTx(Transaction),
    GetPeers,
    Peers(Vec<String>),
}
```

on the wire:
```json
{"type": "new_block",  "data": { ...block... }}
{"type": "get_chain"}
{"type": "chain",      "data": [ ...blocks... ]}
{"type": "new_tx",     "data": { ...tx... }}
```

---

## P2PLayer struct

```rust
pub struct P2PLayer {
    listener: TcpListener,
    peers:    Arc<Mutex<Vec<String>>>,
    state:    Arc<RwLock<NodeState>>,
    mempool:  Arc<Mutex<Mempool>>,
}
```

`peers` is just a `Vec<String>` of `"host:port"` addresses. no scoring, no eviction, no cap. it's a prototype — the peer list grows but never shrinks. good enough for now.

---

## methods

### `new`
binds the TCP listener synchronously, same as the RPC server. fail fast if the port is taken.

### `listen`
converts the std listener to a tokio listener, loops on accept, spawns `handle_peer` per connection. each spawned task gets `Arc` clones of state, mempool, peers.

### `connect`
dials an outbound connection to a known peer address, adds it to the peer list, spawns `handle_peer` on the stream. called at startup if `--peer` was passed.

### `broadcast_block`
opens a fresh TCP connection to every known peer and sends `NewBlock`. each peer is fire-and-forget — if one fails it logs nothing and moves on. a single unreachable peer shouldn't stop the block from reaching the rest.

I open a new connection per broadcast instead of keeping persistent ones. persistent connections mean I have to track health, reconnect on drop, etc. a new connection per message is a bit wasteful but the code stays dead simple.

### `broadcast_tx`
same as `broadcast_block` but sends `NewTx`. called when the RPC receives a transaction and wants to propagate it.

### `sync_with_peer`
used once at startup after `connect`. sends `GetChain`, reads back the `Chain` response, and calls `chain.replace(blocks)` to adopt the peer's chain if it's longer and valid. if sync fails the node just starts with its local chain and catches up from incoming blocks.

one thing to note here — I blindly trust the chain I get back. no cryptographic verification that it actually came from who I think it did. this is fine for a local dev network but would be a problem in the wild.

---

## handle_peer — what happens when a message arrives

one message per connection. read a line, parse it, handle it, done:

| message | what I do |
|---------|-----------|
| `NewBlock` | write-lock state, try `chain.add_block`, if it passes call `apply_block` to update balances. if it fails (block behind or bad hash) I just swallow the error — the node will eventually catch up |
| `GetChain` | read-lock state, clone the blocks, send `Chain(blocks)` back |
| `NewTx` | read state to get sender's on-chain balance, then `mempool.add(tx, sender_balance)`. bad txs are silently dropped |
| `GetPeers` | send back `Peers([])` — peer exchange is not built yet |
| anything else | ignored |

---

## the one-message-per-connection thing

this keeps the code simple. no framing, no partial reads, no "what if the message is split across two reads". you open a connection, you send one thing, you optionally get one thing back, you close. same approach as the RPC server.

the downside is connection overhead per broadcast. fine for 2-10 peers, would need rethinking beyond that.

---

## known rough edges

- `replace()` trusts whatever chain the peer sends — no total-work comparison
- `GetPeers` returns an empty list, so nodes can't discover each other automatically — you have to pass `--peer` manually
- dead peers stay in the list forever and cause silent failures on every broadcast
- no maximum message size — a malicious peer could send a huge line and OOM the node
