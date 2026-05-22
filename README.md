# KamiChain

A minimal but complete proof-of-work blockchain built from scratch in Rust. Nodes discover each other via libp2p, sign transactions with ed25519, and resolve forks via longest chain rule. Interact via CLI.

---

## Architecture

```
kamichain-core    — Block, Chain, Transaction, ProofOfWork
kamichain-wallet  — Ed25519 keypairs, signing, verification
kamichain-node    — Running node: mempool, P2P networking, RPC
kamichain-cli     — CLI client (kami) to interact with a node
```

## Build Order

1. `kamichain-core` — data structures and PoW
2. `kamichain-wallet` — keys and transaction signing
3. `kamichain-node` — mempool, then P2P last
4. `kamichain-cli` — wire everything together

## Development

```bash
# Run all tests
cargo test

# Run only core tests
cargo test -p kamichain-core

# Run only wallet tests
cargo test -p kamichain-wallet
```

Tests are written first. Make them pass.
