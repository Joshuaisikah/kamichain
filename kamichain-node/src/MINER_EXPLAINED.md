# miner.rs — how blocks get made

the miner is pretty simple conceptually. it grabs pending transactions from the mempool, sticks a coinbase reward on the front, runs PoW until the hash looks right, then writes the block to the chain. the tricky part was getting the locking right so mining doesn't freeze the RPC and P2P layers.

---

## constants

```rust
pub const BLOCK_REWARD: u64 = 50;
pub const MAX_TXS_PER_BLOCK: usize = 100;
```

50 coins per block, max 100 user transactions per block (the coinbase doesn't count toward that cap). picked these numbers to be bitcoin-adjacent and easy to test with. reward halving is a future problem.

---

## the struct

```rust
pub struct Miner {
    pub address: String,
    pub pow: ProofOfWork,
}
```

`address` is where the block reward goes. `pow` just holds the difficulty and builds the target prefix string — it's cheap to create but I keep it in the struct so I'm not rebuilding it on every mine.

---

## mine_block — the slow part

```rust
pub fn mine_block(&self, state: &SharedState, mempool: &Mempool) -> Block {
```

this is the CPU-intensive part. I deliberately separated it from the commit so I don't hold any write locks while hashing.

what it does:
1. grabs a read lock just long enough to snapshot `(index, prev_hash)` from the chain tip, then immediately drops it
2. calls `mempool.take(MAX_TXS_PER_BLOCK)` — returns the highest-fee transactions without removing them yet
3. prepends a coinbase tx at position 0 so the miner always gets paid first
4. calls `pow.mine(&mut block)` which just increments `block.nonce` in a loop until `block.hash.starts_with("00...")`

the reason I snapshot and release the read lock before mining: PoW can take seconds at difficulty 4+. holding a read lock that whole time would block anything that needs a write lock (like an incoming block from P2P). drop it early, mine freely.

---

## mine_and_commit — the fast part

```rust
pub fn mine_and_commit(
    &self,
    state: &SharedState,
    mempool: &mut Mempool,
) -> Result<Block, KamiError> {
```

this wraps `mine_block` and then commits atomically:

```
mine_block()        ← can take seconds, no locks held
    ↓
state.write()       ← held for maybe a millisecond
  chain.add_block
  apply_block
  drop write lock
    ↓
mempool.remove      ← clean up confirmed txs
```

the write lock covers `add_block` and `apply_block` together on purpose — I never want a state where the chain has a new block but the balances haven't been updated yet. one lock, both mutations, drop.

if `add_block` fails (wrong index, bad hash, bad PoW) it returns a `KamiError`. in practice this only happens if two miners somehow produce blocks at the same height at the same time. when it happens in tests it's because the test is deliberately feeding a bad block.

---

## the mempool lock situation

the caller in `node.rs` holds the mempool `Mutex` for the entire `mine_and_commit` call, which means the whole mine duration. that's not great — `tx_submit` from RPC will block while mining is happening.

I know this is a problem. the proper fix is to take a snapshot of transactions before mining and lock only for the commit. but that needs deduplication logic to handle the case where the same transaction appears in two simultaneously-mined blocks. keeping it simple for now, will fix when throughput actually matters.

---

## data flow

```
mempool.take(100) ──► mine_block ──PoW──► Block
                                            │
                            ┌───────────────┘
                            │
                      state.write()
                       ├─ chain.add_block
                       └─ apply_block
                            │
                       mempool.remove (confirmed txs)
                            │
                      return block ──► storage.save + p2p.broadcast
```
