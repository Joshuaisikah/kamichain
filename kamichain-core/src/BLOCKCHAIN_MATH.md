# the math behind kamichain

everything crypto in this project runs through SHA-256 from the `sha2` crate. that's it. no fancy curves, no zero-knowledge stuff, just SHA-256 hashed over and over in different combinations. knowing how each piece feeds into the next makes the whole thing click.

---

## SHA-256 — the one tool doing all the work

SHA-256 takes any bytes and spits out a fixed 256-bit digest — 32 bytes, which I write as a 64-char lowercase hex string. three things make it useful here:

- give it the same input twice, you get the same output (obvious but important)
- change one byte of input and the output looks completely different (~50% of bits flip)
- you can't go backwards — knowing the hash doesn't tell you anything about the input

every single hash in this codebase comes down to this exact pattern:

```rust
// merkle.rs
fn hash_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

`format!("{:x}", ...)` is just turning those 32 bytes into the 64-char hex string. that's the whole foundation.

---

## transaction IDs

when I create a transaction I need a unique ID for it. I hash four fields together:

```rust
// transaction.rs
pub fn compute_id(sender: &str, recipient: &str, amount: u64, nonce: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sender.as_bytes());
    hasher.update(recipient.as_bytes());
    hasher.update(amount.to_string().as_bytes());
    hasher.update(nonce.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}
```

`sha2` accumulates the updates before finalising, so this is the same as hashing one big concatenated string. the result is a 64-char hex ID.

the `nonce` is the important one here — it's a random `u64` I generate via `rand::random()` inside `Transaction::new`. without it, alice sending bob 100 twice would produce the same tx ID both times and the second one would look like a duplicate. with a 64-bit random nonce the collision probability is basically zero (1 in 2^64).

the struct method just delegates to the free function using the stored nonce:

```rust
// transaction.rs
pub fn compute_id(&self) -> String {
    compute_id(&self.sender, &self.recipient, self.amount, self.nonce)
}
```

---

## merkle tree — summarising all transactions in one hash

the merkle tree is how I get a single hash that represents every transaction in a block. if you change any transaction, the root changes, which changes the block hash. that's the point.

### step 1 — hash each transaction ID

first I hash every tx ID to get the leaf nodes. same `hash_str` as above:

```rust
// merkle.rs
fn hash_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

### step 2 — hash pairs of nodes together

then I combine adjacent nodes left-to-right:

```rust
// merkle.rs
fn hash_pair(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

order matters — `hash_pair("A","B") != hash_pair("B","A")`. that's intentional and tested in `merkle_tests.rs`.

### step 3 — keep going up until there's one hash left

```rust
// merkle.rs
pub fn new(hashes: Vec<String>) -> Self {
    if hashes.is_empty() {
        let root = hash_str("");
        return MerkleTree { leaves: vec![], root };
    }
    let leaves = hashes.clone();
    let mut level = hashes.iter().map(|h| hash_str(h)).collect::<Vec<_>>();
    while level.len() > 1 {
        let mut next_level = Vec::new();
        let mut i = 0;
        while i < level.len() {
            let left  = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]      // odd node — pair it with itself
            };
            next_level.push(hash_pair(left, right));
            i += 2;
        }
        level = next_level;
    }
    MerkleTree { leaves, root: level[0].clone() }
}
```

visually for 4 transactions it looks like this:

```
leaves:   H(tx0)   H(tx1)   H(tx2)   H(tx3)
               \   /               \   /
level 1:    H(L0+L1)             H(L2+L3)
                    \             /
root:            H(N0+N1)
```

for an odd number of leaves, the last one gets paired with itself:

```
leaves:   H(tx0)   H(tx1)   H(tx2)
               \   /           |
level 1:    H(L0+L1)      H(L2+L2)   ← duplicated
                    \       /
root:          H(N0+N1)
```

empty block gets `SHA-256("")` as the root — a fixed value, not an error.

---

## block hash — committing to everything

each block's hash covers five fields:

```rust
// block.rs
pub fn compute_hash(&self) -> String {
    let input = format!(
        "{}{}{}{}{}",
        self.index, self.timestamp, self.merkle_root, self.prev_hash, self.nonce
    );
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

why each field is in there:

- `index` — makes sure this hash is only valid at this height
- `timestamp` — so two blocks with identical transactions still hash differently
- `merkle_root` — any change to any transaction propagates here
- `prev_hash` — this is the chain link. change it and you break the connection to the parent
- `nonce` — this is the only field the miner actually changes when searching for a valid hash

`is_hash_valid` is just a sanity check — recompute and compare:

```rust
// block.rs
pub fn is_hash_valid(&self) -> bool {
    self.hash == self.compute_hash()
}
```

---

## proof of work — making blocks expensive to produce

the difficulty tells you how many leading hex zeros the block hash needs. difficulty 2 means the hash has to start with `"00"`:

```rust
// pow.rs
pub fn target_prefix(&self) -> String {
    "0".repeat(self.difficulty)
}
```

each hex char is 4 bits, so difficulty `d` means the first `4d` bits have to be zero. the probability of a random hash satisfying that is `1/16^d`:

| difficulty | expected attempts |
|-----------|------------------|
| 1 | 16 |
| 2 | 256 |
| 3 | 4,096 |
| 4 | 65,536 |

the mining loop just keeps incrementing the nonce until it finds a hash that works:

```rust
// pow.rs
pub fn mine(&self, block: &mut Block) {
    let target = self.target_prefix();
    loop {
        block.hash = block.compute_hash();
        if block.hash.starts_with(&target) {
            break;
        }
        block.nonce += 1;
    }
}
```

validation is the flip side — two quick checks:

```rust
// pow.rs
pub fn validate(&self, block: &Block) -> Result<(), KamiError> {
    if block.hash != block.compute_hash() {
        return Err(KamiError::InvalidPoW);
    }
    if !block.hash.starts_with(&self.target_prefix()) {
        return Err(KamiError::InvalidPoW);
    }
    Ok(())
}
```

mining can take seconds. validation takes microseconds. that asymmetry is the whole point of PoW.

---

## chain linking

each block stores the hash of the block before it. the genesis block has a hard-coded sentinel:

```rust
// block.rs
prev_hash: "0".repeat(64),
```

so the chain looks like:

```
block 0            block 1             block 2
prev: 0000...  ──► prev: hash(B0)  ──► prev: hash(B1)
hash: H(B0)        hash: H(B1)         hash: H(B2)
```

`add_block` enforces this before accepting anything:

```rust
// chain.rs
pub fn add_block(&mut self, block: Block) -> Result<(), KamiError> {
    let pow = ProofOfWork::new(self.difficulty);
    pow.validate(&block)?;
    if block.prev_hash != self.latest_block().hash {
        return Err(KamiError::InvalidChain(
            "prev_hash does not match the latest block hash".into()
        ));
    }
    self.blocks.push(block);
    Ok(())
}
```

tamper with any field in block 1, its hash changes, which means block 2's `prev_hash` no longer matches. that breaks block 2, which breaks block 3, all the way to the tip. you can't quietly change history.

---

## full chain validation

`is_valid` walks every block from index 1 and runs four checks:

```rust
// chain.rs
pub fn is_valid(&self) -> Result<(), KamiError> {
    let pow = ProofOfWork::new(self.difficulty);
    for i in 1..self.blocks.len() {
        let current = &self.blocks[i];
        let prev    = &self.blocks[i - 1];

        if current.prev_hash != prev.hash {
            return Err(KamiError::InvalidPoW);
        }

        let tx_ids: Vec<String> = current.transactions
            .iter()
            .map(|tx| tx.compute_id())
            .collect();
        let merkle_root = MerkleTree::new(tx_ids).root();
        if merkle_root != current.merkle_root {
            return Err(KamiError::InvalidChain(
                format!("Block {} has invalid merkle root", i)
            ));
        }

        if !current.is_hash_valid() {
            return Err(KamiError::InvalidChain(
                format!("Block {} has invalid hash", i)
            ));
        }

        pow.validate(current)?;
    }
    Ok(())
}
```

in order: chain is linked → transactions match the stored merkle root → block hash is self-consistent → hash satisfies the PoW target. one failure anywhere stops the whole thing. genesis is skipped — it's the trust anchor.

---

## longest chain rule

when a peer sends a different chain I replace mine only if theirs is longer AND valid:

```rust
// chain.rs
pub fn replace(&mut self, candidate: Vec<Block>) -> bool {
    if candidate.len() <= self.blocks.len() {
        return false;
    }
    let temp = Chain {
        blocks: candidate.clone(),
        difficulty: self.difficulty,
    };
    if temp.is_valid().is_err() {
        return false;
    }
    self.blocks = candidate;
    true
}
```

longer = more PoW was done to produce it. a malicious chain that's longer but invalid still gets rejected at `is_valid()`.

---

## balance arithmetic

once a block is confirmed I update the in-memory balance ledger in `apply_block`:

```rust
// state.rs
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

coinbase just adds coins to the recipient — no sender, money appears from nothing. transfer moves coins from sender to recipient using `saturating_sub` so it clamps at zero instead of panicking or wrapping. the mempool already rejects underfunded transactions so this case shouldn't actually happen — but I don't want an overflow panic if something slips through.

---

## every formula in one place

| what | formula | where |
|------|---------|-------|
| tx ID | `SHA-256(sender ∥ recipient ∥ amount_str ∥ nonce_str)` | `transaction.rs::compute_id` |
| merkle leaf | `SHA-256(tx_id)` | `merkle.rs::hash_str` |
| merkle parent | `SHA-256(left ∥ right)` | `merkle.rs::hash_pair` |
| merkle root (empty block) | `SHA-256("")` | `merkle.rs::new` |
| block hash | `SHA-256(index ∥ timestamp ∥ merkle_root ∥ prev_hash ∥ nonce)` | `block.rs::compute_hash` |
| PoW target | first `difficulty` hex chars must be `"0"` | `pow.rs::target_prefix` |
| difficulty odds | `1 / 16^difficulty` per attempt | `pow.rs` |
| chain link | `block[i].prev_hash == block[i-1].hash` | `chain.rs::add_block` |
| chain replace | longer AND `is_valid()` passes | `chain.rs::replace` |
| coinbase balance | `balances[recipient] += reward` | `state.rs::apply_block` |
| transfer balance | `balances[sender] = sat_sub(balance, amount); balances[recipient] += amount` | `state.rs::apply_block` |
