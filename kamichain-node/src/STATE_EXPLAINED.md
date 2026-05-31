# Node State — `state.rs`

## Purpose

`NodeState` is the single source of truth for the running node. It holds the canonical chain and an in-memory balance ledger derived from that chain. Everything that needs to read or write these two things — the miner, the RPC server, the P2P layer — shares a single `Arc<RwLock<NodeState>>`.

---

## Types

### `SharedState`

```rust
pub type SharedState = Arc<RwLock<NodeState>>;
```

A type alias so callers don't have to spell out `Arc<RwLock<NodeState>>` everywhere. The `RwLock` allows unlimited concurrent readers (RPC `chain_info`, P2P `GetChain`) but exclusive access for writers (miner commit, P2P `NewBlock`).

### `NodeState`

```rust
pub struct NodeState {
    pub chain:    Chain,
    pub balances: HashMap<String, u64>,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `chain` | `Chain` | The ordered list of blocks, with the genesis block at index 0. |
| `balances` | `HashMap<String, u64>` | In-memory ledger: `address → confirmed balance`. Only reflects confirmed (on-chain) transactions. |

---

## Methods

### `new(difficulty) -> NodeState`

Creates a fresh node state: an empty chain (genesis block created by `Chain::new`) and an empty balance map.

### `new_shared(difficulty) -> SharedState`

Convenience constructor that wraps `new()` in an `Arc<RwLock<...>>`. Used in tests and can replace the ad-hoc construction in `node.rs`.

### `apply_block(&mut self, block: &Block)`

Updates the balance ledger for every transaction in the block:

```
TxType::Coinbase
    balances[recipient] += amount
    (no sender deduction — coinbase creates new coins)

TxType::Transfer
    balances[sender]    -= amount   (saturating_sub — never underflows)
    balances[recipient] += amount
```

**Why `saturating_sub`?** It prevents an unsigned integer underflow panic if a block somehow credits more than a sender has. In a correctly-validated chain this case should never arise (the chain's `is_valid` would catch it), but `saturating_sub` is a safety net for the in-memory ledger.

**When to call it**: `apply_block` is called immediately after `chain.add_block` succeeds — in both `miner.mine_and_commit` and `p2p::handle_peer`. The two calls must always be paired; applying a block that was not added to the chain would desync the ledger.

### `balance_of(&self, address: &str) -> u64`

Looks up the confirmed balance for an address. Returns `0` for unknown addresses (no error — they simply have a zero balance).

---

## Consistency Guarantees

The `RwLock` ensures that `chain` and `balances` are always mutated together in the same write guard. A reader that holds a read guard sees a consistent snapshot: the balance ledger always reflects exactly the transactions in the chain at the time the guard was acquired.

```
Thread A (miner)        Thread B (RPC)
─────────────────       ─────────────────
state.write()           state.read()        ← blocked until A releases
  add_block
  apply_block
  drop guard          → unblocked, sees both updates
```

---

## Relationship to Other Components

```
                ┌──────────────────┐
                │   NodeState      │
                │  chain           │◄── Chain::add_block (miner, p2p)
                │  balances        │◄── apply_block      (miner, p2p)
                └──────────────────┘
                        ▲
              Arc<RwLock<NodeState>>
                    ┌───┴────┐
                  Miner    RpcServer    P2PLayer
```

---

## What Is Not in `NodeState`

- **Mempool** — pending (unconfirmed) transactions are tracked separately in `Mempool`. Keeping them separate means the mempool can be drained without holding the chain write lock.
- **Peers** — the P2P layer owns the peer list; it does not need to be visible to the chain or balance logic.
- **Disk persistence** — `Storage` is responsible for serialising `chain` to JSON. `NodeState` is purely in-memory.
