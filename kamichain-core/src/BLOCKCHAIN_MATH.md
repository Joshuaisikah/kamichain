# Blockchain Math — How KamiChain Computes Everything

All cryptographic work in this project uses **SHA-256** from the `sha2` crate.  
SHA-256 produces a 256-bit (32-byte) digest, written as a 64-character lowercase hex string.

---

## 1. SHA-256 — The Foundation

SHA-256 is a one-way function:

```
SHA-256(input bytes) → 64-hex-char string
```

Properties that the whole blockchain relies on:

| Property | What it means for KamiChain |
|----------|------------------------------|
| Deterministic | Same input always gives the same hash |
| Avalanche effect | Changing one byte of input changes ~50% of output bits |
| Preimage resistance | You cannot reverse a hash to find the input |
| Collision resistance | Two different inputs producing the same hash is computationally infeasible |

Every hash in this codebase is produced by the same three-line pattern — here is the exact function from `merkle.rs`:

```rust
// merkle.rs
fn hash_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

`format!("{:x}", ...)` turns the 32-byte digest into a 64-character lowercase hex string.

---

## 2. Transaction ID

**File**: `transaction.rs` — `compute_id()`

```rust
// transaction.rs
pub fn compute_id(sender: &str, recipient: &str, amount: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sender.as_bytes());
    hasher.update(recipient.as_bytes());
    hasher.update(amount.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}
```

The formula is:

```
tx_id = SHA-256( sender_bytes ∥ recipient_bytes ∥ amount_string_bytes )
```

The three fields are fed into the hasher one after another — `sha2` accumulates them before finalising, so the result is identical to hashing a single concatenated byte string.

**Example**:
```
sender    = "alice"
recipient = "bob"
amount    = 100

tx_id = SHA-256("alice" ++ "bob" ++ "100")
      → some 64-char hex string
```

**Limitation**: two transactions from alice to bob for 100 each produce the same ID because there is no nonce or timestamp in the formula. Adding a sequence number to the hash input would fix this.

The method is also available on the struct itself — it delegates to the free function:

```rust
// transaction.rs
pub fn compute_id(&self) -> String {
    compute_id(&self.sender, &self.recipient, self.amount)
}
```

---

## 3. Merkle Tree

**File**: `merkle.rs`

A Merkle tree is a binary tree of hashes. Its **root** is a single hash that summarises every transaction in a block. Changing any transaction changes the root, which then changes the block hash.

### 3a. Leaf hashing

Each transaction ID string is hashed once to produce a leaf node using `hash_str`:

```rust
// merkle.rs
fn hash_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

```
leaf_i = SHA-256( tx_id_i )
```

### 3b. Pair hashing

Adjacent leaves are combined by feeding both hashes into a single hasher — left first, then right:

```rust
// merkle.rs
fn hash_pair(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

```
parent = SHA-256( left_bytes ∥ right_bytes )
```

Order matters here — `hash_pair("A","B") ≠ hash_pair("B","A")`, which is why `order_of_transactions_matters` is a test in `merkle_tests.rs`.

### 3c. Tree construction

Here is the full `MerkleTree::new` from your codebase:

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
                &level[i]      // odd node — duplicate itself
            };
            next_level.push(hash_pair(left, right));
            i += 2;
        }
        level = next_level;
    }
    MerkleTree { leaves, root: level[0].clone() }
}
```

The algorithm:

```
Level 0 (leaves):  H(tx0)   H(tx1)   H(tx2)   H(tx3)
                     \       /           \       /
Level 1:           H(L0∥L1)             H(L2∥L3)
                         \               /
Level 2 (root):       H(L01∥L23)
```

**Odd number of nodes** — the last node is paired with itself:

```
Level 0:  H(tx0)   H(tx1)   H(tx2)
                \   /          |
Level 1:      H(L0∥L1)    H(L2∥L2)   ← L2 duplicated
                    \       /
Level 2 (root):  H(L01∥L22)
```

**Empty block** — when `hashes` is empty, the root is `SHA-256("")` (a defined constant, not an error):

```rust
if hashes.is_empty() {
    let root = hash_str("");
    return MerkleTree { leaves: vec![], root };
}
```

### 3d. Full worked example (4 transactions)

```
tx_ids = ["aaa", "bbb", "ccc", "ddd"]

Leaves (each tx_id hashed once):
  L0 = SHA-256("aaa")
  L1 = SHA-256("bbb")
  L2 = SHA-256("ccc")
  L3 = SHA-256("ddd")

Level 1 (pairs):
  N0 = SHA-256(L0 ∥ L1)
  N1 = SHA-256(L2 ∥ L3)

Root:
  R  = SHA-256(N0 ∥ N1)
```

`R` is stored in `block.merkle_root`.

### 3e. Verification

```rust
// merkle.rs
pub fn verify(&self, tx_hash: &str) -> bool {
    self.leaves.contains(&tx_hash.to_string())
}
```

Checks whether a raw tx_id string is in the original `leaves` vec. This confirms inclusion without recomputing the full tree path.

---

## 4. Block Hash

**File**: `block.rs` — `compute_hash()`

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

The formula:

```
block_hash = SHA-256(
    index_str
  ∥ timestamp_str
  ∥ merkle_root      (64 hex chars — summarises all transactions)
  ∥ prev_hash        (64 hex chars — links to parent block)
  ∥ nonce_str        (the number the miner increments)
)
```

All five fields are formatted into a single `String` before hashing. This means the bytes are concatenated with no separators — the order and types of the fields are fixed by the `format!` call.

**What each field contributes**:

| Field | Type | Why it's in the hash |
|-------|------|----------------------|
| `index` | `u64` | Ties the block to its position in the chain |
| `timestamp` | `u64` (Unix seconds) | Ensures two otherwise-identical blocks produce different hashes |
| `merkle_root` | 64-char hex | Commits to all transactions — changing any tx changes this |
| `prev_hash` | 64-char hex | Links this block to its parent; changing this breaks the chain |
| `nonce` | `u64` | The only field changed during mining |

`is_hash_valid` simply recomputes and compares:

```rust
// block.rs
pub fn is_hash_valid(&self) -> bool {
    self.hash == self.compute_hash()
}
```

---

## 5. Proof of Work

**File**: `pow.rs`

PoW is the puzzle that makes producing a valid block computationally expensive but verifying one cheap.

### 5a. Difficulty and target

```rust
// pow.rs
pub fn target_prefix(&self) -> String {
    "0".repeat(self.difficulty)
}
```

Difficulty `d` means the block hash must begin with `d` hex zeros:

```
difficulty = 2  →  target prefix = "00"
difficulty = 4  →  target prefix = "0000"
```

Each hex character represents 4 bits. Difficulty `d` requires the first `4d` bits of the hash to be zero:

```
difficulty d  →  probability of a random hash satisfying = 1 / 16^d
```

| Difficulty | Expected hashes needed |
|------------|------------------------|
| 1 | 16 |
| 2 | 256 |
| 3 | 4,096 |
| 4 | 65,536 |
| 6 | 16,777,216 |

### 5b. Mining loop

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

On each iteration the nonce is incremented, the hash is recomputed, and the prefix is checked. The nonce is a `u64`, giving 2^64 ≈ 18 quintillion attempts before overflow — far more than any realistic difficulty requires.

### 5c. Validation

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

Two checks:
1. The stored hash matches a fresh recomputation — the block was not tampered with after mining.
2. The hash satisfies the difficulty prefix — the PoW was actually done.

Both are O(1): one SHA-256 call and one string prefix scan. This is why validation is instant while mining takes seconds.

---

## 6. Chain Linking — The Hash Chain

**File**: `chain.rs`

Each block stores the hash of the block before it in `prev_hash`. The genesis block uses a fixed sentinel:

```rust
// block.rs — genesis()
prev_hash: "0".repeat(64),
```

This forms an unbreakable chain:

```
Block 0 (genesis)          Block 1                   Block 2
┌────────────────┐         ┌────────────────┐         ┌────────────────┐
│ prev_hash:     │         │ prev_hash:     │         │ prev_hash:     │
│ "0000...0000"  │    ┌───►│ hash(Block 0)  │    ┌───►│ hash(Block 1)  │
│                │    │    │                │    │    │                │
│ hash: H(B0) ───┼────┘    │ hash: H(B1) ───┼────┘    │ hash: H(B2)    │
└────────────────┘         └────────────────┘         └────────────────┘
```

`add_block` enforces the link before appending:

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

**Why tampering cascades**: change any field in Block 1 → its hash changes → Block 2's `prev_hash` no longer matches → Block 2's own hash changes → Block 3 is broken, and so on to the tip.

---

## 7. Full Chain Validation

**File**: `chain.rs` — `is_valid()`

```rust
// chain.rs
pub fn is_valid(&self) -> Result<(), KamiError> {
    let pow = ProofOfWork::new(self.difficulty);
    for i in 1..self.blocks.len() {
        let current = &self.blocks[i];
        let prev    = &self.blocks[i - 1];

        // 1. chain is linked
        if current.prev_hash != prev.hash {
            return Err(KamiError::InvalidPoW);
        }

        // 2. transactions match the stored merkle root
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

        // 3. stored hash is self-consistent
        if !current.is_hash_valid() {
            return Err(KamiError::InvalidChain(
                format!("Block {} has invalid hash", i)
            ));
        }

        // 4. hash satisfies PoW difficulty
        pow.validate(current)?;
    }
    Ok(())
}
```

Four checks per block, in order:

| Check | Catches |
|-------|---------|
| `prev_hash` linkage | Any break in the chain |
| Merkle root recompute | Transaction list was altered after mining |
| `is_hash_valid` | Block fields were altered after mining |
| `pow.validate` | Block was produced without meeting difficulty |

The genesis block (index 0) is skipped — it is the trust anchor.

---

## 8. Chain Replacement (Longest Chain Rule)

**File**: `chain.rs` — `replace()`

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

Accept the candidate chain only if:
1. It is **strictly longer** than the local chain.
2. It passes **full `is_valid()` validation** (all four checks on every block).

The longer chain wins because it represents more accumulated PoW. A malicious chain that is longer but invalid is rejected at step 2.

---

## 9. Balance Arithmetic

**File**: `state.rs` — `apply_block()`

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

**Coinbase** — new coins enter the system:
```
balances[recipient] += reward
```

**Transfer** — coins move between accounts:
```
balances[sender]    = saturating_sub(balances[sender], amount)
balances[recipient] += amount
```

`saturating_sub` behaviour:
```
if balance >= amount  →  balance - amount   (normal subtraction)
if balance <  amount  →  0                  (clamps, never panics or wraps)
```

A correctly-validated chain never lets a sender overdraw (the mempool rejects underfunded transactions), but `saturating_sub` is a hard safety net against any bug that lets an invalid transaction reach this function.

---

## Summary — Every Formula at a Glance

| Computation | Formula | Source |
|-------------|---------|--------|
| Transaction ID | `SHA-256(sender ∥ recipient ∥ amount_str)` | `transaction.rs::compute_id` |
| Merkle leaf | `SHA-256(tx_id)` | `merkle.rs::hash_str` |
| Merkle parent | `SHA-256(left ∥ right)` | `merkle.rs::hash_pair` |
| Merkle root (empty) | `SHA-256("")` | `merkle.rs::new` |
| Block hash | `SHA-256(index ∥ timestamp ∥ merkle_root ∥ prev_hash ∥ nonce)` | `block.rs::compute_hash` |
| PoW target | `block_hash[0..difficulty] == "0" × difficulty` | `pow.rs::target_prefix` |
| Difficulty odds | `1 / 16^difficulty` per hash attempt | `pow.rs` |
| Chain link | `block[i].prev_hash == block[i-1].hash` | `chain.rs::add_block` |
| Chain replace | `candidate.len() > local.len() AND candidate.is_valid()` | `chain.rs::replace` |
| Coinbase balance | `balances[recipient] += reward` | `state.rs::apply_block` |
| Transfer balance | `balances[sender] = sat_sub(balance, amount); balances[recipient] += amount` | `state.rs::apply_block` |
