# Miner — `miner.rs`

## Purpose

The `Miner` is the component that produces new blocks. It pulls pending transactions out of the mempool, prepends a coinbase reward transaction, runs Proof-of-Work, and then commits the result to the shared chain state — all in one atomic operation.

---

## Constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `BLOCK_REWARD` | `50` | Amount credited to the miner's address per block (coinbase). |
| `MAX_TXS_PER_BLOCK` | `100` | Hard cap on user transactions per block (coinbase not counted). |

---

## `Miner` Struct

```rust
pub struct Miner {
    pub address: String,   // reward destination address
    pub pow: ProofOfWork,  // encapsulates difficulty + hashing loop
}
```

### `Miner::new(address, difficulty)`

Constructs a miner bound to a specific reward address and difficulty level.  
`ProofOfWork::new(difficulty)` pre-computes the zero-prefix string used during mining.

---

## Methods

### `mine_block(&self, state, mempool) -> Block`

Produces a valid block **without** touching the chain yet. Two-phase design:

1. **Read phase** — acquires a short read lock to snapshot `(index, prev_hash)` from the chain tip. The lock is dropped immediately so the mining loop (which can take seconds) never blocks readers.

2. **Assemble transactions** — calls `mempool.take(MAX_TXS_PER_BLOCK)` which returns the highest-fee transactions without removing them. A coinbase transaction is prepended at position 0 so the reward is always the first tx in the block.

3. **Mine** — `Block::new(index, txs, prev_hash)` creates the block shell, then `pow.mine(&mut block)` increments the nonce until the block hash satisfies the difficulty target (correct number of leading zeros).

**Why separate from commit?** Mining is slow (CPU-bound). Holding a write lock during PoW would starve the RPC and P2P layers. Separating the two phases means the chain is write-locked only for a microsecond.

---

### `mine_and_commit(&self, state, mempool) -> Result<Block, KamiError>`

The full pipeline — mine then commit atomically:

```
mine_block()          ← slow, no lock held
    ↓
state.write()         ← fast: add_block + apply_block
    ↓
mempool.remove(txs)   ← clean up confirmed transactions
```

1. Calls `mine_block()` (can take seconds, no lock held).
2. Acquires a single write lock and:
   - Calls `chain.add_block(block.clone())` — validates linkage and difficulty, appends to the chain.
   - Calls `state.apply_block(&block)` — updates the in-memory balance ledger.
3. Removes the confirmed transactions from the mempool (including the coinbase, which is already filtered out in `mempool.add`).

**Returns** the mined block so the caller (`node.rs`) can persist it and broadcast it to peers.

**Error cases** — `chain.add_block` returns `KamiError` if the block is invalid (wrong index, hash mismatch, bad difficulty). In practice this only happens under a race where two miners produce blocks at the same height simultaneously.

---

## Data Flow in Context

```
Mempool ──take(100)──► mine_block ──PoW──► Block
                                              │
                              ┌───────────────┘
                              │
                        state.write()
                         ├─ chain.add_block
                         └─ apply_block (update balances)
                              │
                         mempool.remove
                              │
                        return block ──► storage.save + p2p.broadcast
```

---

## Threading Model

`mine_and_commit` takes `&self` and `&mut Mempool`. The caller in `node.rs` holds the `mempool` `Mutex` guard for the duration of the call. This is intentional: it prevents two concurrent mining attempts from both pulling the same transactions and racing on the write lock.

The downside is that the `Mutex` is held during the entire mining duration. In production you would mine outside the lock and only lock for the commit phase — but that requires deduplication logic for the case where the same transaction appears in two simultaneously-mined blocks.
