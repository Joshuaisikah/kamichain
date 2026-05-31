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

Every hash in this codebase is produced by the same two-line pattern:
```rust
let mut hasher = Sha256::new();
hasher.update(some_bytes);
format!("{:x}", hasher.finalize())   // → 64 lowercase hex chars
```

---

## 2. Transaction ID

**File**: `transaction.rs` — `compute_id()`

```
tx_id = SHA-256( sender ∥ recipient ∥ amount_as_string )
```

Where `∥` means byte concatenation in the order the fields are fed to the hasher:
```rust
hasher.update(sender.as_bytes());
hasher.update(recipient.as_bytes());
hasher.update(amount.to_string().as_bytes());
```

**Example** (pseudocode):
```
sender    = "alice"
recipient = "bob"
amount    = 100

tx_id = SHA-256("alice" ++ "bob" ++ "100")
      = "e3b0c4..." (64 hex chars)
```

**Limitation**: two transactions from alice to bob for 100 units will produce the same ID. In production this is fixed by including a nonce or timestamp in the ID computation — this codebase keeps it simple.

---

## 3. Merkle Tree

**File**: `merkle.rs`

A Merkle tree is a binary tree of hashes. Its **root** is a single hash that summarises every transaction in a block. Changing any transaction changes the root, which then changes the block hash.

### 3a. Leaf hashing

Each transaction ID string is hashed once to produce a leaf:

```
leaf_i = SHA-256( tx_id_i )
```

```rust
fn hash_str(s: &str) -> String {
    SHA-256(s.as_bytes())
}
```

### 3b. Pair hashing

Adjacent leaves are combined by feeding both into a single hasher (left then right):

```
parent = SHA-256( left ∥ right )
```

```rust
fn hash_pair(left: &str, right: &str) -> String {
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    SHA-256(hasher.finalize())
}
```

### 3c. Tree construction

Starting from the leaf level, the algorithm walks up level by level until a single root remains:

```
Level 0 (leaves):  H(tx0)   H(tx1)   H(tx2)   H(tx3)
                     \       /           \       /
Level 1:           H(L0∥L1)             H(L2∥L3)
                         \               /
Level 2 (root):       H(L01∥L23)
```

**Odd number of nodes**: if a level has an odd count, the last node is paired with itself:

```
Level 0:  H(tx0)   H(tx1)   H(tx2)
                \   /          |
Level 1:      H(L0∥L1)    H(L2∥L2)   ← L2 duplicated
                    \       /
Level 2 (root):  H(L01∥L22)
```

This is the standard Bitcoin Merkle tree approach to handle odd counts.

**Empty block**: if there are no transactions, the root is:
```
root = SHA-256("")
```

### 3d. Full worked example (4 transactions)

```
tx_ids = ["aaa", "bbb", "ccc", "ddd"]

Leaves:
  L0 = SHA-256("aaa")
  L1 = SHA-256("bbb")
  L2 = SHA-256("ccc")
  L3 = SHA-256("ddd")

Level 1:
  N0 = SHA-256(L0 ∥ L1)
  N1 = SHA-256(L2 ∥ L3)

Root:
  R  = SHA-256(N0 ∥ N1)
```

The root `R` is stored in `block.merkle_root`.

### 3e. Verification

`MerkleTree::verify(tx_hash)` checks whether a raw tx_id string exists in the original `leaves` vec — a simple membership test. This confirms a transaction was included without recomputing the full tree.

---

## 4. Block Hash

**File**: `block.rs` — `compute_hash()`

The block hash commits to all of its identifying fields in one SHA-256 call:

```
block_hash = SHA-256(
    index_str
  ∥ timestamp_str
  ∥ merkle_root
  ∥ prev_hash
  ∥ nonce_str
)
```

Implemented as:
```rust
let input = format!(
    "{}{}{}{}{}",
    self.index, self.timestamp, self.merkle_root, self.prev_hash, self.nonce
);
SHA-256(input.as_bytes())
```

**What each field contributes**:

| Field | Type | Why it's in the hash |
|-------|------|----------------------|
| `index` | `u64` | Ties the block to its position in the chain |
| `timestamp` | `u64` (Unix seconds) | Ensures two otherwise-identical blocks have different hashes |
| `merkle_root` | 64-char hex | Commits to all transactions — changing any tx changes this |
| `prev_hash` | 64-char hex | Links this block to its parent; breaking the link invalidates all descendant blocks |
| `nonce` | `u64` | The mining counter — the only field the miner changes during PoW |

---

## 5. Proof of Work

**File**: `pow.rs`

PoW is the puzzle that makes producing a valid block computationally expensive but verifying one cheap.

### 5a. Difficulty and target

Difficulty `d` means the block hash must begin with `d` hex zeros:

```
difficulty = 2  →  target prefix = "00"
difficulty = 4  →  target prefix = "0000"
```

Each hex character represents 4 bits. Difficulty `d` therefore requires the first `4d` bits of the hash to be zero:

```
difficulty d  →  probability of a random hash satisfying = 1 / 16^d
```

| Difficulty | Expected hashes needed | Approximate |
|------------|----------------------|-------------|
| 1 | 16^1 | 16 |
| 2 | 16^2 | 256 |
| 3 | 16^3 | 4,096 |
| 4 | 16^4 | 65,536 |
| 6 | 16^6 | 16,777,216 |

### 5b. Mining loop

```
nonce = 0
loop:
    hash = SHA-256(index ∥ timestamp ∥ merkle_root ∥ prev_hash ∥ nonce)
    if hash.starts_with("0" × difficulty):
        FOUND — block.hash = hash, break
    nonce += 1
```

```rust
pub fn mine(&self, block: &mut Block) {
    let target = "0".repeat(self.difficulty);
    loop {
        block.hash = block.compute_hash();
        if block.hash.starts_with(&target) { break; }
        block.nonce += 1;
    }
}
```

The nonce is a `u64`, giving 2^64 ≈ 18 quintillion attempts before overflow. In practice difficulty is low enough that a valid hash is found in hundreds to thousands of tries.

### 5c. Validation

Validation runs two checks:

```
1.  block.hash == SHA-256(block fields)      ← hash is internally consistent
2.  block.hash.starts_with("0" × difficulty) ← hash satisfies the target
```

```rust
pub fn validate(&self, block: &Block) -> Result<(), KamiError> {
    if block.hash != block.compute_hash() { return Err(KamiError::InvalidPoW); }
    if !block.hash.starts_with(&self.target_prefix()) { return Err(KamiError::InvalidPoW); }
    Ok(())
}
```

Both checks are O(1) — SHA-256 of a short string, then a prefix scan. This is why PoW validation is fast while mining is slow.

---

## 6. Chain Linking — The Hash Chain

**File**: `chain.rs`

Each block stores the hash of the block before it in `prev_hash`. This forms an unbreakable chain:

```
Block 0 (genesis)          Block 1                   Block 2
┌────────────────┐         ┌────────────────┐         ┌────────────────┐
│ prev_hash:     │         │ prev_hash:     │         │ prev_hash:     │
│ "0000...0000"  │    ┌───►│ hash(Block 0)  │    ┌───►│ hash(Block 1)  │
│                │    │    │                │    │    │                │
│ hash: H(B0) ───┼────┘    │ hash: H(B1) ───┼────┘    │ hash: H(B2)    │
└────────────────┘         └────────────────┘         └────────────────┘
```

The genesis block uses `"0".repeat(64)` as its `prev_hash` — a sentinel value indicating there is no parent.

**Why tampering is caught**: if you change any field in Block 1, its hash changes. Block 2's `prev_hash` no longer matches Block 1's new hash. Block 2's hash then changes, breaking Block 3, and so on — the entire chain from the tampered block onward is invalidated.

### `add_block` validation

Before appending, two invariants are checked:

```
1.  PoW valid:  block.hash starts with "0" × difficulty AND matches recomputed hash
2.  Linkage:    block.prev_hash == chain.latest_block().hash
```

---

## 7. Full Chain Validation

**File**: `chain.rs` — `is_valid()`

Walks every block from index 1 (skipping genesis) and for each block checks four properties in order:

```
For each block i (i = 1 … n):

  1.  blocks[i].prev_hash == blocks[i-1].hash          ← chain is linked
  2.  MerkleTree(tx_ids).root() == blocks[i].merkle_root ← transactions unchanged
  3.  blocks[i].hash == SHA-256(block fields)           ← hash is self-consistent
  4.  blocks[i].hash.starts_with("0" × difficulty)     ← PoW satisfied
```

All four must pass for every block. A single failure returns an error immediately.

**Note**: the genesis block (index 0) is not validated — it is trusted as the hard-coded starting point.

---

## 8. Chain Replacement (Longest Chain Rule)

**File**: `chain.rs` — `replace()`

When a peer sends a different chain, KamiChain uses the longest valid chain rule:

```
accept candidate chain if and only if:
  candidate.len() > local.len()     ← strictly longer
  AND candidate.is_valid() == Ok    ← passes full validation
```

```rust
pub fn replace(&mut self, candidate: Vec<Block>) -> bool {
    if candidate.len() <= self.blocks.len() { return false; }
    let temp = Chain { blocks: candidate.clone(), difficulty: self.difficulty };
    if temp.is_valid().is_err() { return false; }
    self.blocks = candidate;
    true
}
```

This is the same rule Bitcoin uses: the chain with the most accumulated work (approximated here by length) wins. A longer chain implies more PoW was done to produce it, making it the authoritative history.

---

## 9. Balance Arithmetic

**File**: `state.rs` — `apply_block()`

Balances are plain `u64` values (unsigned 64-bit integers, 0 to 18,446,744,073,709,551,615).

```
Coinbase transaction:
  balances[recipient] += amount           (new coins created)

Transfer transaction:
  balances[sender]    -= amount           (saturating — never wraps below 0)
  balances[recipient] += amount
```

`saturating_sub` is used for the sender deduction:

```rust
*sender_balance = sender_balance.saturating_sub(tx.amount);
```

```
saturating_sub(a, b):
  if a >= b → a - b
  if a <  b → 0       (clamps at zero, no panic, no wrap)
```

A correctly-validated chain should never put a sender into deficit (signature + balance checks would prevent it), but `saturating_sub` is a safety net against any bug that lets an invalid transaction reach `apply_block`.

---

## Summary — Every Formula at a Glance

| Computation | Formula |
|-------------|---------|
| Transaction ID | `SHA-256(sender ∥ recipient ∥ amount_str)` |
| Merkle leaf | `SHA-256(tx_id)` |
| Merkle parent | `SHA-256(left_child ∥ right_child)` |
| Merkle root (empty) | `SHA-256("")` |
| Block hash | `SHA-256(index ∥ timestamp ∥ merkle_root ∥ prev_hash ∥ nonce)` |
| PoW target | `block_hash[0..difficulty] == "000…0"` |
| Difficulty odds | `1 / 16^difficulty` per hash attempt |
| Chain link | `block[i].prev_hash == block[i-1].hash` |
| Chain replace | `candidate.len() > local.len() AND candidate.is_valid()` |
| Coinbase balance | `balances[recipient] += reward` |
| Transfer balance | `balances[sender] = sat_sub(balance, amount); balances[recipient] += amount` |
