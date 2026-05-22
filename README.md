# KamiChain

A minimal but complete proof-of-work blockchain built from scratch in Rust.
Nodes discover each other over TCP, sign transactions with ed25519, and resolve
forks via longest-chain rule. Interact via the `kami` CLI.

---

## Architecture

```
kamichain/
├── LICENSE
├── docs/
│   ├── architecture.md      # Design decisions and data-flow diagrams
│   └── protocol.md          # Full RPC and P2P wire-format spec
│
├── kamichain-core/          # Block, Chain, Transaction, Merkle, ProofOfWork
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs         # KamiError
│   │   ├── transaction.rs   # Transaction, TxType (Coinbase | Transfer)
│   │   ├── block.rs         # Block — index, timestamp, merkle_root, prev_hash, hash, nonce
│   │   ├── merkle.rs        # MerkleTree — root, verify (inclusion proof)
│   │   ├── pow.rs           # ProofOfWork — mine (multi-threaded via Rayon), validate
│   │   └── chain.rs         # Chain — add_block, is_valid, replace (fork resolution)
│   ├── benches/
│   │   └── chain_bench.rs   # Criterion benchmarks: hashing, mining, validation
│   └── tests/
│       ├── block_tests.rs
│       ├── chain_tests.rs
│       ├── merkle_tests.rs
│       ├── pow_tests.rs
│       └── transaction_tests.rs
│
├── kamichain-wallet/        # Ed25519 keypairs, signing, verification
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs         # WalletError
│   │   └── wallet.rs        # Wallet — new, address, sign_transaction, verify_transaction
│   └── tests/
│       └── wallet_tests.rs
│
├── kamichain-node/          # Running node — mempool, miner, RPC, P2P, storage
│   ├── src/
│   │   ├── lib.rs
│   │   ├── state.rs         # NodeState — Chain + balance ledger, Arc<RwLock<>>
│   │   ├── mempool.rs       # Mempool — pending tx pool with capacity limit
│   │   ├── miner.rs         # Miner — parallel nonce search (Rayon), mine_and_commit
│   │   ├── rpc.rs           # RpcServer — newline-JSON over TCP
│   │   ├── p2p.rs           # P2PLayer — gossip, chain sync, peer exchange
│   │   ├── storage.rs       # Storage — persist/load chain to disk (atomic write)
│   │   └── bin/
│   │       └── node.rs      # Binary — flags, starts RPC + P2P + mining loop
│   └── tests/
│       ├── mempool_tests.rs
│       ├── miner_tests.rs
│       ├── state_tests.rs
│       ├── storage_tests.rs
│       ├── rpc_tests.rs
│       ├── p2p_tests.rs
│       └── e2e_tests.rs     # Golden-path: wallet → tx → mine → confirm → balance
│
└── kamichain-cli/           # `kami` binary — talks to a node over RPC
    └── src/
        ├── main.rs
        └── commands/
            ├── mod.rs
            ├── wallet.rs    # kami wallet new | address | balance
            ├── tx.rs        # kami tx send | get
            ├── chain.rs     # kami chain info | block | validate
            └── node.rs      # kami node start | peers | sync
```

---

## How the pieces fit together

```
[kami CLI] ──RPC/TCP──► [kamichain-node]
                               │
                    ┌──────────┼──────────┐
                    ▼          ▼          ▼
                [Chain]   [Mempool]   [P2P peers]
                    │
              [kamichain-core]
                    │
              [kamichain-wallet]
```

1. The node starts, initialises `NodeState` (chain + balances) and the mempool.
2. The miner loop takes transactions from the mempool, prepends a coinbase tx,
   runs PoW, commits the block, and broadcasts it to peers.
3. Peers that receive a new block validate it and add it to their chain.
   If a peer has a longer valid chain, `Chain::replace` adopts it.
4. The RPC server handles CLI requests — submit tx, query balance, inspect chain.

---

## Build order

```
1. kamichain-core     → transaction → block → pow → chain
2. kamichain-wallet   → error → wallet
3. kamichain-node     → state → mempool → miner → rpc → p2p → bin/node.rs
4. kamichain-cli      → commands → main.rs
```

## Running the tests

```bash
# all tests
cargo test --workspace

# one crate at a time
cargo test -p kamichain-core
cargo test -p kamichain-wallet
cargo test -p kamichain-node

# benchmarks (run after implementing)
cargo bench -p kamichain-core
```

Tests will not compile until you define the public API. They are the spec —
make them pass.

## Running a node

```bash
# start a node on default port
cargo run --bin kamichain-node -- --bind 0.0.0.0:8332 --difficulty 4

# connect a second node to it
cargo run --bin kamichain-node -- --bind 0.0.0.0:8333 --difficulty 4 --peer 127.0.0.1:8332

# use the CLI against the first node
cargo run --bin kami -- wallet new
cargo run --bin kami -- chain info
cargo run --bin kami -- mine --address <your-address>
```

---

## What this demonstrates

| Concept | Where |
|---------|-------|
| Workspace with multiple crates | `Cargo.toml` |
| Custom error types (`thiserror`) | `error.rs` in each crate |
| SHA-256 hashing | `block.rs`, `transaction.rs` |
| Merkle tree | `merkle.rs` |
| Proof-of-work mining | `pow.rs` |
| Multi-threaded nonce search (Rayon) | `miner.rs` |
| Ed25519 signatures | `wallet.rs` |
| `Arc<RwLock<>>` shared state | `state.rs` |
| Chain persistence (atomic file writes) | `storage.rs` |
| Async TCP server (`tokio`) | `rpc.rs`, `p2p.rs` |
| Newline-delimited JSON protocol | `p2p.rs`, `docs/protocol.md` |
| Fork resolution (longest chain) | `chain.rs` |
| Integration + end-to-end test suite | `tests/` in each crate |
| Criterion benchmarks | `kamichain-core/benches/` |
| CI (build, test, clippy, fmt) | `.github/workflows/ci.yml` |
