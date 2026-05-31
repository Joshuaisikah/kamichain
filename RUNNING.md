# running kamichain end-to-end

this is the practical guide — build it, start a node, send coins, watch the chain grow. no theory, just commands.

---

## build everything first

```bash
cargo build --workspace
```

this compiles all four crates. the two binaries land at:

```
target/debug/kamichain-node   ← the node
target/debug/kami             ← the CLI
```

add them to your PATH for the session or just use `cargo run` as shown below.

---

## run all the tests

```bash
cargo test --workspace
```

should be green across all crates. if anything fails here, fix it before running the node.

run just one crate:
```bash
cargo test -p kamichain-core
cargo test -p kamichain-wallet
cargo test -p kamichain-node
```

---

## single node — the simplest setup

open a terminal and start the node:

```bash
cargo run -p kamichain-node -- \
  --bind 127.0.0.1:8333 \
  --rpc  127.0.0.1:8332 \
  --difficulty 2 \
  --data-dir ./data \
  --miner my_miner_address
```

or with the kami CLI:

```bash
cargo run --bin kami -- node start \
  --difficulty 2 \
  --miner my_miner_address
```

you should see:
```
KamiChain Node
  bind:       127.0.0.1:8333
  rpc:        127.0.0.1:8332
  difficulty: 2
  data dir:   ./data
  miner:      my_miner_address
RPC listening on 127.0.0.1:8332
P2P listening on 127.0.0.1:8333
Mining started...
Mined block 1 — hash: 00a3f12b
Mined block 2 — hash: 00e91cd4
...
```

leave this running. open a second terminal for the CLI commands.

---

## create a wallet

```bash
cargo run --bin kami -- wallet new --keyfile alice.key
```

output:
```
address : a3f1b2c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2
keyfile : alice.key
```

the file `alice.key` now holds your private key as a hex string. keep it safe. your address is the SHA-256 of your public key.

check the address any time:
```bash
cargo run --bin kami -- wallet address --keyfile alice.key
```

---

## check balance

```bash
cargo run --bin kami -- wallet balance <your-address>
```

a fresh wallet starts at 0. the miner address you passed to `--miner` earns 50 coins per block. if you used `my_miner_address` as the miner, check that:

```bash
cargo run --bin kami -- wallet balance my_miner_address
```

after a few blocks this returns something like:
```
balance : 150
```

---

## check chain info

```bash
cargo run --bin kami -- chain info
```

```
height      : 4
latest hash : 00c3f1e2d4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1
difficulty  : 2
```

inspect a specific block:
```bash
cargo run --bin kami -- chain block 1
```

prints the full block JSON — index, timestamp, transactions, merkle_root, prev_hash, hash, nonce.

validate the whole chain:
```bash
cargo run --bin kami -- chain validate
```

```
✓ chain is valid
```

if anything is broken it prints what went wrong and exits with code 1.

---

## send a transaction

first the sender needs coins. easiest way is to mine with the sender's real address. create alice's wallet, start the node with her address as the miner, let it mine a few blocks, then she has balance.

```bash
# create alice's wallet
cargo run --bin kami -- wallet new --keyfile alice.key

# get alice's address
ALICE=$(cargo run --bin kami -- wallet address --keyfile alice.key)

# create bob's wallet
cargo run --bin kami -- wallet new --keyfile bob.key
BOB=$(cargo run --bin kami -- wallet address --keyfile bob.key)

# start the node, mining to alice
cargo run -p kamichain-node -- --miner $ALICE --difficulty 2
```

wait for a few blocks. alice now has coins. in another terminal:

```bash
# send 10 from alice to bob, fee 1
cargo run --bin kami -- tx send \
  --keyfile alice.key \
  --to $BOB \
  --amount 10 \
  --fee 1
```

output:
```
submitted : a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4
```

the transaction is now in the mempool. it gets confirmed in the next mined block. check balances after:

```bash
cargo run --bin kami -- wallet balance $BOB
# balance : 10

cargo run --bin kami -- wallet balance $ALICE
# balance : <reward * blocks> - 10
```

look up the transaction by ID:
```bash
cargo run --bin kami -- tx get <the-id-from-submitted>
```

---

## two-node setup (fork resolution)

open three terminals.

**terminal 1 — node A:**
```bash
cargo run -p kamichain-node -- \
  --bind 127.0.0.1:8333 \
  --rpc  127.0.0.1:8332 \
  --difficulty 2 \
  --data-dir ./data-a \
  --miner miner_a
```

**terminal 2 — node B (connects to A on startup):**
```bash
cargo run -p kamichain-node -- \
  --bind 127.0.0.1:8335 \
  --rpc  127.0.0.1:8334 \
  --difficulty 2 \
  --data-dir ./data-b \
  --miner miner_b \
  --peer 127.0.0.1:8333
```

node B downloads node A's chain on startup and starts mining from the same tip. when either mines a block it broadcasts to the other. both chains stay in sync.

**terminal 3 — CLI against node A:**
```bash
cargo run --bin kami -- chain info
# or against node B:
cargo run --bin kami -- chain info --node 127.0.0.1:8334
```

both should show the same or near-same height. if B temporarily gets ahead (longer chain), A adopts B's chain on the next block broadcast via `chain.replace()`.

---

## running the benchmarks

```bash
cargo bench -p kamichain-core
```

runs the Criterion benchmarks for block hashing, merkle root (100 txs), PoW mining at difficulty 2, and chain validation (10 blocks). the parallel mining in `pow.rs` means the PoW bench benefits from your core count — run it at difficulty 4 to see a more meaningful difference between cores:

edit `chain_bench.rs` and change the difficulty to 4, then run again. the parallel version should be noticeably faster on multi-core machines.

---

## cleaning up between runs

the node persists the chain to `./data/chain.json` (or whatever `--data-dir` you set). to start fresh:

```bash
rm -rf ./data ./data-a ./data-b
```

---

## common errors

**"connection refused" from the CLI**
the node isn't running, or it's on a different port. check `--node` flag matches the node's `--rpc` address.

**"insufficient balance"**
the sender's on-chain balance is too low. wait for more blocks to be mined, or restart with that address as the miner.

**"Sync failed: ..."**
node B couldn't download node A's chain at startup. node B will start with its own genesis and both nodes mine independently. they'll re-sync when one broadcasts a new block.

**Mining never produces blocks**
difficulty is too high for the machine. use `--difficulty 2` (expects ~256 hash attempts per block).
