# state.rs — the node's in-memory truth

`NodeState` is the one place that holds both the chain and the balance ledger. everything else — miner, RPC, P2P — shares a single `Arc<RwLock<NodeState>>` and talks to it through that.

---

## why one shared struct

I could have kept the chain and balances separate, but they need to stay in sync. if I update the chain without updating balances, or vice versa, things go wrong. wrapping them in one struct behind one `RwLock` means I can update both atomically in a single write guard.

```rust
pub type SharedState = Arc<RwLock<NodeState>>;

pub struct NodeState {
    pub chain:    Chain,
    pub balances: HashMap<String, u64>,
}
```

`SharedState` is just a type alias so I don't have to write `Arc<RwLock<NodeState>>` everywhere. the `RwLock` lets multiple readers go at the same time (RPC serving `chain_info` while P2P handles `GetChain`), but writes are exclusive.

---

## constructors

```rust
pub fn new(difficulty: usize) -> Self
pub fn new_shared(difficulty: usize) -> SharedState
```

`new` gives you a plain `NodeState` with a genesis chain and empty balances. `new_shared` wraps it in the `Arc<RwLock<...>>` ready to pass around. I mostly use `new_shared` in tests, `new` in the binary where I build the state manually from a loaded chain.

---

## apply_block

this is where confirmed transactions actually move money:

```rust
pub fn apply_block(&mut self, block: &Block) {
    for tx in &block.transactions {
        match tx.tx_type {
            TxType::Coinbase => {
                *self.balances.entry(tx.recipient.clone()).or_insert(0) += tx.amount;
            }
            TxType::Transfer => {
                let sender_balance = self.balances
                    .entry(tx.sender.clone())
                    .or_insert(0);
                *sender_balance = sender_balance.saturating_sub(tx.amount);
                *self.balances.entry(tx.recipient.clone()).or_insert(0) += tx.amount;
            }
        }
    }
}
```

coinbase just adds coins to the recipient — that's how new money enters the system. transfer debits the sender and credits the recipient.

`saturating_sub` for the debit means if for some reason the sender's balance is lower than the transfer amount it clamps to zero instead of underflowing. the mempool already checks `amount + fee <= balance` before admitting transactions, so this shouldn't happen on a clean chain. but bugs happen and I'd rather clamp than panic.

**important**: `apply_block` must always be called right after `chain.add_block` succeeds, and never called on a block that wasn't added. the miner does this, the P2P layer does this. if you ever call one without the other the chain and balances go out of sync and you'll have a bad time.

---

## balance_of

```rust
pub fn balance_of(&self, address: &str) -> u64 {
    *self.balances.get(address).unwrap_or(&0)
}
```

returns 0 for any address that's never received anything. no error. this is what the RPC `wallet_balance` method and the mempool balance check both use.

---

## locking in practice

```
Thread A (miner commits a block)    Thread B (RPC serves chain_info)
────────────────────────────────    ────────────────────────────────
state.write()                       state.read()    ← waits
  chain.add_block
  apply_block
  drop guard                      → gets read lock, sees consistent state
```

the reader always sees the chain and balances at the same point in time. no partial updates.

---

## what's NOT in NodeState

- **mempool** — kept separate so the miner can drain it without holding the chain write lock
- **peer list** — that's the P2P layer's business
- **disk** — `Storage` handles serialising the chain to JSON. `NodeState` is purely in-memory and gets rebuilt from the file on startup
