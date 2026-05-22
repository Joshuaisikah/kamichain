# Architecture

## Crate dependency graph

```
kamichain-cli
    ├── kamichain-core
    └── kamichain-wallet

kamichain-node
    ├── kamichain-core
    └── kamichain-wallet

kamichain-wallet
    └── kamichain-core

kamichain-core   (no internal deps)
```

`kamichain-core` is the foundation — no async, no networking, no crypto. Pure data structures and algorithms. Every other crate depends on it.

---

## Why these design decisions

### Proof-of-Work over Proof-of-Stake
PoW is simpler to implement correctly. The consensus rule is one line: the valid chain is the one with the most cumulative work. No validator sets, no slashing, no staking contracts. For a from-scratch implementation PoW lets you focus on the blockchain mechanics rather than incentive design.

### SHA-256 for hashing
SHA-256 is the battle-tested standard used by Bitcoin. `sha2` is a well-audited Rust crate with no unsafe code. Every block hash and transaction ID uses SHA-256.

### Merkle tree for transaction hashing
Blocks store a Merkle root of their transactions rather than hashing the raw list. This enables:
- **Tamper detection** — changing any single transaction changes the root and therefore the block hash
- **SPV proofs** — a light client can verify a transaction is in a block with O(log n) hashes, without downloading the full block

### Ed25519 for transaction signing
Ed25519 (`ed25519-dalek`) over ECDSA/secp256k1 because:
- Faster signing and verification
- Smaller keys and signatures (32-byte public key, 64-byte signature)
- Deterministic — same key + same message always produces the same signature
- Resistant to fault attacks that can leak ECDSA private keys

### `Arc<RwLock<NodeState>>` for shared state
The node runs three concurrent tasks: miner loop, RPC server, P2P layer. All three need access to the chain and balance ledger. `Arc<RwLock<>>` gives:
- `Arc` — shared ownership across threads without copying
- `RwLock` — multiple concurrent readers (RPC queries), exclusive writer (miner commits a block)

### Newline-delimited JSON for the wire protocol
Simple to implement, debug, and extend. Every message is one line of JSON terminated by `\n`. No framing complexity, no binary encoding to get wrong. Readable with `nc` or `curl`.

### Chain::replace for fork resolution
When a peer broadcasts a longer chain, `Chain::replace` accepts it if and only if:
1. The candidate is strictly longer than the current chain
2. `is_valid()` passes on the full candidate chain

This is the longest-chain rule — the same fork resolution used by Bitcoin.

---

## Data flow: mining a block

```
Mempool::take(MAX_TXS_PER_BLOCK)
    │
    ▼
Miner::mine_block
    ├── prepend Transaction::coinbase(miner_addr, BLOCK_REWARD)
    ├── set block.prev_hash = chain.latest_block().hash
    └── ProofOfWork::mine(&mut block)
            └── loop: block.nonce += 1 until hash starts with "0" * difficulty
    │
    ▼
Chain::add_block(block)
    ├── ProofOfWork::validate  →  reject if hash wrong
    ├── check prev_hash link   →  reject if broken
    └── push to chain.blocks
    │
    ▼
NodeState::apply_block(block)
    ├── credit coinbase recipient
    └── for each transfer: debit sender, credit recipient
    │
    ▼
Mempool::remove confirmed tx IDs
    │
    ▼
P2PLayer::broadcast_block(block)
```

---

## Data flow: receiving a block from a peer

```
P2PLayer receives Message::NewBlock(block)
    │
    ▼
Chain::add_block(block)         ← same validation as local mining
    │ success
    ▼
NodeState::apply_block(block)
    │
    ▼
Mempool: remove any txs now confirmed
```

If the peer's chain is longer (detected via `Message::Chain`), `Chain::replace` runs instead.

---

## Multi-threaded mining

The miner splits the nonce search space across CPU cores using Rayon. Each thread checks a different nonce range in parallel. The first thread to find a valid hash sends it via a channel; the others abort.

```
rayon::spawn × num_cpus
    each thread: try nonces in [thread_id * RANGE .. (thread_id+1) * RANGE]
    first hit   → send block via oneshot channel
    others      → check AtomicBool::found, break early
```

---

## Persistence

`Storage` serialises the full `Chain` to JSON and writes it atomically (write to `.tmp`, then rename). On startup, the node calls `Storage::load_chain()` — if the file exists the chain is restored, otherwise a fresh genesis chain is created.
