# bin/node.rs — the entry point

`node.rs` is intentionally thin. it parses config, creates all the shared state, wires the subsystems together, and then sits in the mining loop forever. all the actual logic lives in the library modules — this file just plugs them together.

---

## startup order

```
1.  NodeConfig::from_args()         parse flags, validate, apply defaults
2.  fs::create_dir_all(data_dir)    make sure the data dir exists
3.  Storage::new(chain_path)        set up the storage layer
4.  storage.load_chain()            load from disk, or start fresh with genesis
5.  Arc<RwLock<NodeState>>          wrap the chain in shared state
6.  Arc<Mutex<Mempool>>             shared mempool
7.  RpcServer::new + tokio::spawn   bind the RPC port, start accepting
8.  P2PLayer::new                   bind the P2P port
9.  p2p.connect + sync_with_peer    if --peer was given, connect and sync
10. tokio::spawn p2p.listen()       start accepting incoming P2P connections
11. Miner::new                      set up the miner
12. loop { mine_and_commit }        mine forever
```

the order matters in a few places. P2P is created before connecting to a peer so the listen socket is ready before we dial out — avoids a race where a peer tries to connect back to us before we're listening. RPC is spawned before P2P so HTTP clients can hit the node during the P2P sync.

---

## config flags

all flags have defaults so `kamichain-node` with no arguments just works:

| flag | default | what it does |
|------|---------|-------------|
| `--bind` | `127.0.0.1:8333` | P2P port |
| `--rpc` | `127.0.0.1:8332` | RPC port |
| `--difficulty` | `2` | PoW difficulty |
| `--data-dir` | `./data` | where chain.json lives |
| `--miner` | `default_miner` | coinbase reward address |
| `--peer` | *(nothing)* | bootstrap peer to sync from on startup |

passing an unknown flag or a flag without a value exits immediately with a clear error message. see `config.rs` for the parsing logic.

example:
```bash
kamichain-node \
  --bind 0.0.0.0:8333 \
  --rpc  0.0.0.0:8332 \
  --difficulty 4 \
  --data-dir /var/kamichain \
  --miner my_wallet_address \
  --peer 192.168.1.10:8333
```

---

## the two shared things

```rust
let state   = Arc::new(RwLock::new(NodeState { chain, balances: HashMap::new() }));
let mempool = Arc::new(Mutex::new(Mempool::new(10_000)));
```

created once, then `Arc::clone`'d into every subsystem that needs them. `Arc` is just shared ownership — every clone points to the same allocation. `RwLock` on state allows concurrent readers (RPC + P2P can both serve reads at the same time), `Mutex` on mempool is exclusive (one writer at a time).

---

## how each subsystem starts

**RPC** — binds the port right in `new()`. if the port is in use we panic immediately. moved into a `tokio::spawn` so it runs concurrently.

**P2P** — same deal. bind in `new()`, spawn `listen()` after the optional peer sync. the reason I wrap it in `Arc::new(p2p)` right before spawning is that `broadcast_block` in the mining loop needs a reference to it after `listen()` takes ownership.

**peer sync** — if `--peer` was passed I connect and call `sync_with_peer` which sends `GetChain` and adopts the peer's chain if it's longer. failure here is non-fatal — just log it and keep going with the local chain.

---

## the mining loop

```rust
loop {
    let mut mempool_guard = mempool.lock().unwrap();
    match miner.mine_and_commit(&state, &mut mempool_guard) {
        Ok(block) => {
            drop(mempool_guard);
            // save to disk
            // broadcast to peers
        }
        Err(e) => {
            drop(mempool_guard);
            eprintln!("Mining error: {}", e);
        }
    }
}
```

this blocks the tokio main thread, which is fine — PoW is CPU work, not I/O. the RPC and P2P handlers are running on other tokio tasks so they keep working while this thread hashes.

the mempool mutex is held for the whole mine duration. I know this blocks `tx_submit` RPC calls while mining. it's a known tradeoff — fixing it properly requires taking a snapshot before mining and handling the case where a transaction shows up in two simultaneous blocks. leaving it for later.

I save before broadcasting. if the process crashes between those two steps the block is on disk and will get re-broadcast after a restart. if it crashes after broadcast but before save, peers have the block and the node will re-sync from them on startup.

---

## how errors are handled

- bad config flag → `process::exit(1)`. no point continuing.
- port already in use → panic in `RpcServer::new` / `P2PLayer::new`. operator needs to fix this before starting.
- peer sync fails → log warning, keep going with local chain.
- mining error → log to stderr, keep going. one bad block doesn't kill the node.
- save fails → log to stderr, keep going. the in-memory state is still correct.
