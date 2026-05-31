# Node Binary — `bin/node.rs`

## Purpose

`node.rs` is the entry point for the `kamichain-node` executable. It parses configuration, wires together all subsystems, and runs the main mining loop. It is intentionally thin: all logic lives in the library modules; the binary's only job is to assemble them.

---

## Startup Sequence

```
1.  NodeConfig::from_args()         parse CLI flags → NodeConfig
2.  fs::create_dir_all(data_dir)    ensure data directory exists
3.  Storage::new(chain_path)        connect storage layer
4.  storage.load_chain()            load chain from disk, or create genesis
5.  Arc<RwLock<NodeState>>          wrap chain in shared state
6.  Arc<Mutex<Mempool>>             create shared mempool
7.  RpcServer::new + tokio::spawn   bind RPC port, spawn accept loop
8.  P2PLayer::new                   bind P2P port
9.  p2p.connect + sync_with_peer    (if --peer provided) bootstrap sync
10. tokio::spawn p2p.listen()       spawn P2P accept loop
11. Miner::new                      create miner
12. loop { mine_and_commit }        mining loop (runs forever)
```

---

## Configuration

Parsed by `NodeConfig::from_args()` (see `config.rs`). All flags have defaults so you can run a node with zero arguments.

| Flag | Default | Description |
|------|---------|-------------|
| `--bind <addr>` | `127.0.0.1:8333` | P2P listen address |
| `--rpc <addr>` | `127.0.0.1:8332` | RPC listen address |
| `--difficulty <n>` | `2` | PoW difficulty (leading zero nibbles) |
| `--data-dir <path>` | `./data` | Directory for `chain.json` |
| `--miner <address>` | `default_miner` | Coinbase reward destination |
| `--peer <addr>` | *(none)* | Bootstrap peer to connect to on startup |

If an unknown flag or a missing value is detected, the process prints an error and exits with code 1.

**Example invocations**:
```bash
# minimal — all defaults
kamichain-node

# full configuration
kamichain-node \
  --bind 0.0.0.0:8333 \
  --rpc  0.0.0.0:8332 \
  --difficulty 4       \
  --data-dir /var/kamichain \
  --miner 0xYourAddress \
  --peer 192.168.1.10:8333
```

---

## Shared Resources

Two shared objects are created once and cloned into every subsystem via `Arc`:

| Object | Type | Shared with |
|--------|------|-------------|
| `state` | `Arc<RwLock<NodeState>>` | RPC, P2P, miner |
| `mempool` | `Arc<Mutex<Mempool>>` | RPC, P2P, miner |

`Arc` provides shared ownership across threads. `RwLock` allows the state to be read concurrently (RPC + P2P can both serve chain queries at the same time) but written exclusively (the miner or an incoming P2P block can modify the chain one at a time).

---

## Subsystem Startup

### RPC Server

```rust
let rpc = RpcServer::new(&cfg.rpc_addr, Arc::clone(&state), Arc::clone(&mempool));
tokio::spawn(async move { rpc.run().await.unwrap(); });
```

`RpcServer::new` binds the port synchronously — if the port is taken the process exits here. The server is moved into a tokio task so it runs concurrently with P2P and mining.

### P2P Layer

```rust
let p2p = P2PLayer::new(&cfg.bind_addr, Arc::clone(&state), Arc::clone(&mempool));
```

Created before peer connection so the listen socket is ready before we dial out (avoids a race where the remote peer tries to connect back to us before we're listening).

### Peer Bootstrap

```rust
if let Some(ref peer_addr) = cfg.peer {
    p2p.connect(peer_addr).await.unwrap();
    p2p.sync_with_peer(peer_addr).await.unwrap_or_else(|e| {
        eprintln!("Sync failed: {}", e);
    });
}
```

`connect` adds the peer to the peer list and opens a handler. `sync_with_peer` sends `GetChain` to download the peer's full chain. Sync failure is non-fatal — the node starts with its local (possibly shorter) chain and will catch up as new blocks arrive.

### P2P Listener

```rust
let p2p = Arc::new(p2p);
let p2p_listener = Arc::clone(&p2p);
tokio::spawn(async move { p2p_listener.listen().await.unwrap(); });
```

`p2p` is wrapped in `Arc` at this point so both the listener task and the mining loop (which calls `p2p.broadcast_block`) can hold a reference.

---

## Mining Loop

```rust
loop {
    let mut mempool_guard = mempool.lock().unwrap();
    match miner.mine_and_commit(&state, &mut mempool_guard) {
        Ok(block) => {
            drop(mempool_guard);
            // 1. persist chain to disk
            // 2. broadcast block to all peers
        }
        Err(e) => {
            drop(mempool_guard);
            eprintln!("Mining error: {}", e);
        }
    }
}
```

The loop runs synchronously on the tokio main thread. This is intentional: mining is CPU-bound (PoW) and blocking the async executor on this thread is acceptable because all I/O-bound work (RPC, P2P) runs on separate tokio tasks.

**Why hold the mempool lock for the whole mine duration?** It prevents two code paths from both picking up the same transactions. The trade-off is that `tx_submit` RPC calls will block while a block is being mined. For a low-throughput blockchain this is acceptable; a production node would mine outside the lock and use a snapshot instead.

**Persist before broadcast**: `storage.save_chain` is called before `p2p.broadcast_block`. If the process crashes between save and broadcast, the block is safe on disk and will be retransmitted when the node restarts. If the process crashes after broadcast but before save, peers already have the block — the node will re-sync from them on restart.

---

## Error Handling Philosophy

- **Configuration errors** (`NodeConfig::from_args`) → exit 1. There is no point starting with wrong configuration.
- **Bind failures** (`RpcServer::new`, `P2PLayer::new`) → panic via `unwrap`. The port is in use or the address is invalid; the operator must fix this before the node can run.
- **Sync failure** → warning + continue. The node falls back to its local chain.
- **Mining errors** → log to stderr + continue. A single bad block does not kill the node.
- **Save errors** → log to stderr + continue. The in-memory state is still consistent; the next successful mine will overwrite the stale file.
