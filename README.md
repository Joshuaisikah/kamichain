# KamiChain

A minimal but complete proof-of-work blockchain built from scratch in Rust.
Nodes discover each other over TCP, sign transactions with ed25519, and resolve
forks via longest-chain rule. Interact via the `kami` CLI.

---

## Architecture

```
kamichain/
├── kamichain-core/          # Block, Chain, Transaction, ProofOfWork
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs         # KamiError — one error type for the whole crate
│   │   ├── transaction.rs   # Transaction, TxType (Coinbase | Transfer)
│   │   ├── block.rs         # Block — index, timestamp, txs, prev_hash, hash, nonce
│   │   ├── pow.rs           # ProofOfWork — mine, validate, difficulty target
│   │   └── chain.rs         # Chain — add_block, is_valid, replace (fork resolution)
│   └── tests/
│       ├── block_tests.rs
│       ├── chain_tests.rs
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
├── kamichain-node/          # Running node — mempool, miner, RPC, P2P
│   ├── src/
│   │   ├── lib.rs
│   │   ├── state.rs         # NodeState — Chain + balance ledger, wrapped in Arc<RwLock<>>
│   │   ├── mempool.rs       # Mempool — pending tx pool with capacity limit
│   │   ├── miner.rs         # Miner — mine_block, mine_and_commit, BLOCK_REWARD
│   │   ├── rpc.rs           # RpcServer — JSON over TCP, chain/tx/wallet/peers endpoints
│   │   ├── p2p.rs           # P2PLayer — gossip protocol, chain sync, peer discovery
│   │   └── bin/
│   │       └── node.rs      # Binary entrypoint — parse flags, start RPC + P2P + mining loop
│   └── tests/
│       ├── state_tests.rs
│       ├── mempool_tests.rs
│       ├── miner_tests.rs
│       ├── rpc_tests.rs
│       └── p2p_tests.rs
│
└── kamichain-cli/           # `kami` binary — talks to a node over RPC
    ├── src/
    │   ├── main.rs
    │   └── commands/
    │       ├── mod.rs
    │       ├── wallet.rs    # kami wallet new | address | balance
    │       ├── tx.rs        # kami tx send | get
    │       └── chain.rs     # kami chain info | block | validate
    └── (no integration tests — use kami against a live node)
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
| Proof-of-work mining | `pow.rs` |
| Ed25519 signatures | `wallet.rs` |
| `Arc<RwLock<>>` shared state | `state.rs` |
| Async TCP server (`tokio`) | `rpc.rs`, `p2p.rs` |
| Newline-delimited JSON protocol | `p2p.rs` |
| Fork resolution (longest chain) | `chain.rs` |
| Integration test suite | `tests/` in each crate |
| CI (build, test, clippy, fmt) | `.github/workflows/ci.yml` |
