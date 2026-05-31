# kamichain-cli — how the CLI is built

the CLI is the user-facing layer. it doesn't contain any blockchain logic — it just parses commands with clap, talks to a running node over TCP, and prints results. four command groups: `wallet`, `tx`, `chain`, `node`.

---

## the structure

```
src/
  main.rs          ← clap setup, top-level dispatch
  rpc.rs           ← TCP client helper (send one request, read one response)
  commands/
    mod.rs         ← re-exports the four modules
    wallet.rs      ← kami wallet new | address | balance
    tx.rs          ← kami tx send | get
    chain.rs       ← kami chain info | block | validate
    node.rs        ← kami node start
```

---

## main.rs — clap wiring

```rust
#[derive(Parser)]
#[command(name = "kami", about = "KamiChain CLI")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Wallet(wallet::WalletArgs),
    Tx(tx::TxArgs),
    Chain(chain::ChainArgs),
    Node(node::NodeArgs),
}
```

clap's derive macros handle all the argument parsing. each variant owns its own `Args` struct defined in its command module. `main` just calls `Cli::parse()` and dispatches to the right `run()` function.

`#[tokio::main]` because the RPC calls are async TCP — open connection, write, read, close.

---

## rpc.rs — the TCP client

```rust
pub async fn call(addr: &str, method: &str, params: Value) -> Result<Value> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = stream.into_split();

    let mut line = serde_json::to_string(&json!({ "method": method, "params": params }))?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    let mut reader = BufReader::new(reader);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    let v: Value = serde_json::from_str(response.trim())?;
    if v["ok"].as_bool() != Some(true) {
        bail!("{}", v["error"].as_str().unwrap_or("node returned an error"));
    }
    Ok(v["result"].clone())
}
```

one function does everything: connect, send one JSON line, read one JSON line back, unwrap the result or bail with the error message. the node closes the connection after each response so there's nothing to clean up.

every command that talks to the node calls this. if the node is not running it fails with "connection refused" immediately.

`--node 127.0.0.1:8332` is the default across all commands that need a node. pass `--node` to point at a different address.

---

## commands/wallet.rs

three subcommands:

**`kami wallet new --keyfile <path>`**
calls `Wallet::new()` which generates a random ed25519 keypair, saves the private key hex to the keyfile, and prints the address. nothing network-related here — purely local.

```
address : a3f1b2...
keyfile : wallet.key
```

**`kami wallet address --keyfile <path>`**
loads the keyfile and prints the address. useful for scripts that need to pass the address to other commands.

**`kami wallet balance <address> --node <addr>`**
RPC call → `wallet_balance` → reads `result["balance"]` → prints it. the node looks up the address in its in-memory balance ledger (confirmed on-chain transactions only, mempool not counted).

---

## commands/tx.rs

**`kami tx send`**

```rust
let wallet  = Wallet::load_from_file(&keyfile)?;
let mut tx  = Transaction::new(wallet.address(), &to, amount, fee);
wallet.sign_transaction(&mut tx)?;
rpc::call(&node, "tx_submit", json!({ "tx": tx })).await?;
println!("submitted : {}", tx_id);
```

loads the keyfile, creates a `Transaction` with a random nonce (so every send produces a unique ID even if you send the same amount to the same address twice), signs it with the private key, submits to the node's mempool. the node validates signature and balance before accepting.

the transaction stays in the mempool until the miner picks it up. higher `--fee` means it gets picked first (mempool sorts by fee descending).

**`kami tx get <id>`**

RPC call → `tx_get` → the node searches all confirmed blocks for a transaction with that ID → prints the full transaction JSON. returns an error if the transaction isn't confirmed yet (still in mempool) or doesn't exist.

---

## commands/chain.rs

**`kami chain info`**
RPC → `chain_info` → prints height, latest hash, difficulty. quick sanity check that the node is running and mining.

**`kami chain block <index>`**
RPC → `chain_block` → returns the full block JSON, pretty-printed. block 0 is genesis.

**`kami chain validate`**
RPC → `chain_validate` → the node runs `chain.is_valid()` which re-checks every block's PoW, hash links, and merkle roots. returns `{ "valid": true/false, "message": "..." }`. if invalid the CLI exits with code 1 — useful for scripting.

---

## commands/node.rs — node start

this one is different from the rest. instead of talking to a node over RPC it IS the node. it imports from `kamichain-node` directly and runs the same startup sequence as the `kamichain-node` binary:

```
create data dir
load or create chain
wrap in Arc<RwLock<NodeState>>
start RPC server  → tokio::spawn
start P2P layer
optionally connect to --peer and sync
spawn P2P listen loop
run mining loop forever (blocks the thread)
```

the reason both `kami node start` and `kamichain-node` exist: the binary is for running a dedicated node, the CLI command is for convenience during development ("I just want to start a node without remembering all the flags").

they're identical in behaviour — `node.rs` in the CLI just hardwires the same defaults as `config.rs` in the node binary.

---

## default values

every command that contacts a node defaults to `--node 127.0.0.1:8332`. this matches the node's default `--rpc 127.0.0.1:8332`. if you run the node on a different port, pass `--node <addr>` to every CLI command.

keyfile defaults to `wallet.key` in the current directory. you can have multiple wallets by always specifying `--keyfile`.

---

## what the CLI does NOT do

- it doesn't validate transactions itself — that's the node's job (mempool + signature verification)
- it doesn't track nonces per address — each `Transaction::new` generates a random nonce
- it doesn't wait for confirmation — `tx send` submits and exits; use `tx get <id>` to check if it's confirmed
- it doesn't manage peers — `node_peers` returns an empty list because peer exchange isn't implemented yet
