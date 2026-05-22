# kamichain-node

The running node. Owns the chain, mempool, miner, RPC server, and P2P layer. Build in this order.

## What to build

### `src/state.rs` — shared node state
`NodeState` holds the `Chain` and a balance ledger (`HashMap<address, u64>`).
Wrap it in `Arc<RwLock<NodeState>>` so every subsystem can share it safely across threads.

- `apply_block(block)` — credit the coinbase recipient, debit senders and credit recipients for all transfers
- `balance_of(address)` — return current balance, 0 if unknown

### `src/mempool.rs` — pending transactions
`Mempool` is a `HashMap<tx_id, Transaction>` with a max capacity.

- `add(tx)` — reject duplicates and overflow
- `remove(tx_id)` — called after a tx is confirmed in a block
- `take(max)` — return up to `max` transactions for inclusion in the next block (does not remove them — `remove` is called separately after the block is committed)

### `src/miner.rs` — mining loop
`Miner` holds the miner's address and a `ProofOfWork`.

- `mine_block(state, mempool)` — build a candidate block: prepend a coinbase tx, take pending txs from the mempool, set `prev_hash` from the chain, then mine
- `mine_and_commit(state, mempool)` — mine then call `chain.add_block`, apply the block to state, remove confirmed txs from the mempool

Constant `BLOCK_REWARD = 50`.
Constant `MAX_TXS_PER_BLOCK = 100`.

### `src/rpc.rs` — JSON over TCP
Minimal request/response server. Each connection sends one JSON request, gets one JSON response.

Endpoints:
```
chain/info        → { height, latest_hash, difficulty }
chain/block/:id   → Block JSON
tx/submit         → add a signed transaction to the mempool
wallet/balance    → { address, balance }
node/peers        → list of connected peer addresses
```

### `src/p2p.rs` — peer-to-peer networking (build last)
Newline-delimited JSON over TCP. Message types:

```
{ "type": "new_block",  "block": <Block JSON> }
{ "type": "get_chain" }
{ "type": "chain",      "blocks": [...] }
{ "type": "new_tx",     "tx": <Transaction JSON> }
{ "type": "get_peers" }
{ "type": "peers",      "addrs": ["ip:port", ...] }
```

When a new block is mined, broadcast it. When a peer announces a block, validate and add it. When a peer has a longer chain, call `Chain::replace`.

## Binary: `kamichain-node`

Accepts `--bind`, `--difficulty`, and `--peer` flags. Starts RPC and P2P listeners, then enters the mining loop.

## Tests

```bash
cargo test -p kamichain-node
```
