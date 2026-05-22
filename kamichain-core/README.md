# kamichain-core

The foundation of KamiChain. No networking, no wallet — just the data structures and algorithms.

## What to build

### `src/error.rs`
One `KamiError` enum covering all failure cases: bad PoW, broken hash links, invalid transactions, serialization failures.

### `src/transaction.rs`
A `Transaction` struct with:
- `id` — SHA-256 of the transaction content (sender + recipient + amount)
- `tx_type` — `Coinbase` or `Transfer`
- `sender`, `recipient`, `amount`
- `signature` — set later by the wallet, `None` by default

Two constructors: `Transaction::new(sender, recipient, amount)` and `Transaction::coinbase(recipient, reward)`.

### `src/block.rs`
A `Block` struct with:
- `index`, `timestamp`, `nonce`
- `transactions: Vec<Transaction>`
- `prev_hash` — hex hash of the previous block
- `hash` — SHA-256 of (index + timestamp + nonce + prev_hash + serialized transactions)

`Block::genesis()` produces the fixed first block with `prev_hash = "0" * 64`.
`compute_hash()` must be deterministic — same fields always produce the same hash.

### `src/pow.rs`
`ProofOfWork` holds a `difficulty` (number of leading zeros required).
- `mine(&self, block: &mut Block)` — increment `nonce` until `block.hash` starts with `difficulty` zeros
- `validate(&self, block: &Block)` — check the stored hash matches `compute_hash()` and satisfies difficulty

### `src/chain.rs`
`Chain` owns a `Vec<Block>` and a `difficulty`.
- `new(difficulty)` — starts with the genesis block
- `add_block(block)` — reject if PoW invalid or `prev_hash` doesn't match latest
- `is_valid()` — walk every block, verify hash links and PoW
- `replace(candidate)` — accept `candidate` only if it's longer than the current chain AND passes `is_valid()`

## Tests

Integration tests live in `tests/`. Run them with:

```bash
cargo test -p kamichain-core
```

All tests are failing until you implement the above.
