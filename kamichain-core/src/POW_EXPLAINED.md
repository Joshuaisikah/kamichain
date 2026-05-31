# pow.rs — proof of work, now parallel

proof of work is the puzzle that makes mining expensive. you have to find a nonce that makes the block's SHA-256 hash start with a certain number of zeros. the more zeros required (higher difficulty), the more hashes you have to try on average before you find one that works.

the original implementation was single-threaded — one nonce at a time, one core, loop forever. this works but wastes every other core on the machine. the parallel version does the same thing, just across all available cores using Rayon.

---

## why it needed a rewrite, not just a for_each

the obvious naive approach — `(0..u64::MAX).par_iter().for_each(|n| { block.nonce = n; ... })` — doesn't work. you can't hand `&mut Block` to multiple threads. Rust won't allow it and the borrow checker will tell you exactly why: mutable references can't be shared.

the solution is to separate the read from the write:

1. **snapshot** all the immutable fields out of the block before touching threads
2. **search** nonces in parallel — each thread hashes a different nonce against the same snapshot
3. **write** the winning nonce back to the block once, on the main thread

no shared mutable state during the search. every thread reads the same snapshot and writes nothing. completely safe.

---

## the chunk strategy

I don't just throw all of `0..u64::MAX` at Rayon at once. that would mean Rayon spinning up threads for 18 quintillion nonces. instead I search in chunks of 100,000:

```rust
const CHUNK: u64 = 100_000;
```

the outer loop advances one chunk at a time. for each chunk, Rayon searches all 100,000 nonces in parallel across your cores. the moment any thread finds a valid hash, `find_any` signals all other threads in that chunk to stop. if the whole chunk yields nothing, move to the next chunk.

```
chunk 0:  nonces 0 – 99,999       → searched in parallel, nothing found
chunk 1:  nonces 100,000 – 199,999 → searched in parallel, found at 147,832 → stop
```

at difficulty 2 you expect to find a valid hash in ~256 attempts, so almost always in chunk 0. at difficulty 6 (~16M attempts on average) you're searching ~160 chunks. each chunk is parallelised so the wall-clock time scales with `expected_attempts / num_cores` rather than `expected_attempts`.

---

## the implementation

```rust
pub fn mine(&self, block: &mut Block) {
    let target    = self.target_prefix();
    let index     = block.index;
    let timestamp = block.timestamp;
    let merkle    = block.merkle_root.clone();
    let prev      = block.prev_hash.clone();

    let nonce = (0u64..)
        .step_by(CHUNK as usize)
        .find_map(|start| {
            (start..start.saturating_add(CHUNK))
                .into_par_iter()
                .find_any(|&n| {
                    hash_candidate(index, timestamp, &merkle, &prev, n)
                        .starts_with(&target)
                })
        })
        .expect("nonce space exhausted");

    block.nonce = nonce;
    block.hash  = block.compute_hash();
}
```

**`(0u64..).step_by(CHUNK as usize)`** — produces 0, 100_000, 200_000, ... indefinitely. outer sequential loop over chunk start positions.

**`find_map`** — runs the closure on each chunk start. returns the first `Some(value)` it gets back. if `find_any` returns `None` (no valid nonce in this chunk), `find_map` moves to the next chunk.

**`(start..start.saturating_add(CHUNK)).into_par_iter()`** — converts the nonce range for this chunk into a Rayon parallel iterator. Rayon splits this across your CPU cores automatically.

**`find_any`** — searches the chunk in parallel. returns `Some(nonce)` as soon as any thread finds a nonce whose hash satisfies the target. all other threads stop. the "any" means it might not return the *lowest* valid nonce in the chunk — it returns whichever thread wins the race. that's fine, any valid nonce is acceptable.

**write-back** — once the winning nonce is found, we write it to `block.nonce` and call `block.compute_hash()` to set `block.hash`. the final call to `compute_hash` verifies everything is consistent — the stored hash is computed from the actual block fields including the nonce we just wrote.

---

## hash_candidate — the helper

```rust
fn hash_candidate(index: u64, timestamp: u64, merkle: &str, prev: &str, nonce: u64) -> String {
    let input = format!("{}{}{}{}{}", index, timestamp, merkle, prev, nonce);
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    format!("{:x}", h.finalize())
}
```

this mirrors `Block::compute_hash` exactly — same field order, same format string, same SHA-256 call. I extracted it as a free function so the parallel closure doesn't need to borrow the block at all. each thread call creates its own local `Sha256` hasher, no shared state.

if `Block::compute_hash` ever changes its field order, this function needs to change too. they have to stay in sync or the mining loop will find a nonce that doesn't actually produce the expected hash.

---

## what the threads actually see

inside the parallel closure, all captured variables are either `Copy` or shared references:

```rust
|&n| hash_candidate(index, timestamp, &merkle, &prev, n)
//                  ^u64   ^u64        ^&str    ^&str
//                  Copy   Copy        immutable refs
```

`index` and `timestamp` are `u64` — `Copy`, cheaply duplicated per thread. `merkle` and `prev` are `&str` — shared immutable references, `Sync + Send`, perfectly safe to read from any thread simultaneously. `n` is the nonce from the parallel iterator — each thread gets its own value. no thread touches anything another thread is writing to.

---

## validate — unchanged

```rust
pub fn validate(&self, block: &Block) -> Result<(), KamiError> {
    if block.hash != block.compute_hash() {
        return Err(KamiError::InvalidPoW);
    }
    if !block.hash.starts_with(&self.target_prefix()) {
        return Err(KamiError::InvalidPoW);
    }
    Ok(())
}
```

validation is still single-threaded and O(1) — one SHA-256 call, one prefix check. it doesn't matter whether the block was mined with one thread or forty. the result is the same and validation is already instant.

---

## the tests

the original 7 tests all still pass unchanged — the public API (`pow.mine(&mut block)`) is identical. the parallel rewrite is completely internal. I added 6 more tests specifically for the parallel behaviour:

| test | what it checks |
|------|---------------|
| `parallel_mine_nonce_is_stored_on_block` | after mine, `block.hash == block.compute_hash()` — the nonce and hash are consistent |
| `parallel_mine_at_difficulty_4_is_valid` | higher difficulty forces multiple Rayon chunks, result is still valid |
| `parallel_mine_two_independent_blocks_are_both_valid` | two blocks mined back to back are both independently valid |
| `parallel_mine_different_prev_hashes_give_different_nonces` | different block contents produce different hashes, both valid |
| `parallel_mine_hash_is_64_hex_chars` | output format is correct |
| `validate_rejects_block_with_correct_prefix_but_wrong_hash` | a fake hash can't fool validate even if it starts with the right prefix |

---

## bench

there's a Criterion benchmark at `benches/chain_bench.rs::bench_pow_difficulty_2`. run it before and after this change to see the actual speedup on your machine:

```bash
cargo bench -p kamichain-core --bench chain_bench
```

the speedup depends on how many cores you have. on a 4-core machine at difficulty 2 the gain is small (expected ~256 hashes, search completes in under a millisecond either way). the real gain shows at difficulty 4+ where the expected search is 65,000+ hashes and the wall-clock time drops roughly proportionally to core count.
