# Implementation Guide

Work through this in order. Each step has a test command — run it before moving on.
The tests in `tests/` are the spec. They won't compile until you define the types they import.

**Global rules that apply everywhere:**
- Every public struct and enum must derive `#[derive(Debug, Clone, Serialize, Deserialize)]` so that storage, RPC, and P2P serialization work without extra boilerplate.
- All structs with public fields should also derive `PartialEq` where tests compare them.
- `kamichain-core/src/lib.rs` already contains `pub use` re-exports — you do **not** need to add them; they let tests import `kamichain_core::{Block, Chain, Transaction, …}` directly.

---

## Step 1 — `kamichain-core/src/error.rs`

Define `KamiError` with variants for every failure mode in the crate:
- `InvalidPoW` — hash doesn't satisfy difficulty
- `InvalidChain` — broken hash link or bad PoW in chain
- `InvalidTransaction` — malformed tx
- `BlockNotFound` — index out of range
- `Serialization` — wraps `serde_json::Error`

Use `thiserror::Error`. All other modules return `Result<_, KamiError>`.

```bash
# nothing to test yet — this is just types
cargo check -p kamichain-core
```

---

## Step 2 — `kamichain-core/src/transaction.rs`

Define:
```rust
pub enum TxType { Coinbase, Transfer }

pub struct Transaction {
    pub id: String,
    pub tx_type: TxType,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub fee: u64,                  // miner tip; 0 by default and for coinbase
    pub pub_key: Option<String>,   // hex-encoded verifying key; set by sign_transaction
    pub signature: Option<String>,
}
```

`Transaction::new(sender, recipient, amount)` — set `id = compute_id()`, type = Transfer, `fee = 0`, `pub_key = None`, `signature = None`  
`Transaction::coinbase(recipient, reward)` — sender = `""`, type = Coinbase, `fee = 0`, `pub_key = None`  
`compute_id()` — SHA-256 of `sender + recipient + amount.to_string()`, returned as lowercase hex. **Fee is not part of the ID** — it is informational metadata for mempool ordering only.  
`is_coinbase()` — checks `tx_type == TxType::Coinbase`

```bash
cargo test -p kamichain-core --test transaction_tests
```

---

## Step 3 — `kamichain-core/src/merkle.rs`

Define:
```rust
pub struct MerkleTree { /* internal tree of hashes */ }
```

`MerkleTree::new(hashes: Vec<String>)` — build the tree bottom-up:
- Hash each leaf with SHA-256
- Pair adjacent nodes and hash their concatenation
- If odd number of nodes, duplicate the last one
- Repeat until one root remains

`root() -> String` — return the root hash as 64-char lowercase hex  
`verify(tx_hash: &str) -> bool` — return true if the hash is a leaf in the tree

For an empty input return the SHA-256 of an empty string (or all-zero hash — your choice, be consistent).

```bash
cargo test -p kamichain-core --test merkle_tests
```

---

## Step 4 — `kamichain-core/src/block.rs`

Define:
```rust
pub struct Block {
    pub index: u64,
    pub timestamp: u64,       // Unix seconds
    pub transactions: Vec<Transaction>,
    pub merkle_root: String,  // from MerkleTree::new(tx ids).root()
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u64,
}
```

`Block::genesis()` — index 0, empty txs, prev_hash = "0" * 64, timestamp = fixed constant  
`Block::new(index, txs, prev_hash)` — set timestamp = now, compute merkle_root from tx ids  
`compute_hash()` — SHA-256 of `index + timestamp + merkle_root + prev_hash + nonce` as a JSON or concatenated string. **Must be deterministic.**  
`is_hash_valid()` — `self.hash == self.compute_hash()`

```bash
cargo test -p kamichain-core --test block_tests
```

---

## Step 5 — `kamichain-core/src/pow.rs`

Define:
```rust
pub struct ProofOfWork { pub difficulty: usize }
```

`new(difficulty)` — store difficulty  
`target_prefix()` — `"0".repeat(difficulty)`  
`mine(&self, block: &mut Block)` — loop: `block.nonce += 1`, recompute hash, break when hash starts with target prefix. Set `block.hash`.  
`validate(&self, block: &Block)` — check `block.hash == block.compute_hash()` AND `block.hash.starts_with(&self.target_prefix())`

**Parallelisation (do after single-threaded works):** split the nonce range across `rayon::current_num_threads()` threads. Use an `AtomicBool` to signal when one thread finds the answer.

```bash
cargo test -p kamichain-core --test pow_tests
```

---

## Step 6 — `kamichain-core/src/chain.rs`

Define:
```rust
pub struct Chain {
    pub blocks: Vec<Block>,
    pub difficulty: usize,
}
```

`new(difficulty)` — push `Block::genesis()`  
`latest_block()` — `&self.blocks.last().unwrap()`  
`len()` — `self.blocks.len()`  
`get_block(index: u64)` — `self.blocks.get(index as usize)`  
`add_block(block)`:
  1. `pow.validate(&block)` — reject if PoW wrong
  2. `block.prev_hash == latest_block().hash` — reject if link broken
  3. push  

`is_valid()`:
  - Walk every block from index 1
  - Check `blocks[i].prev_hash == blocks[i-1].hash`
  - Check `blocks[i].hash == blocks[i].compute_hash()`
  - Check PoW on every block  

`replace(candidate: Vec<Block>) -> bool`:
  - Return false if `candidate.len() <= self.blocks.len()`
  - Build a temp chain, call `is_valid()` on it
  - Return false if invalid
  - Replace `self.blocks`, return true

```bash
cargo test -p kamichain-core --test chain_tests
cargo test -p kamichain-core   # all core tests
```

---

## Step 7 — `kamichain-wallet/src/error.rs`

Define `WalletError`:
- `InvalidPublicKey(String)`
- `VerificationFailed`
- `MissingSignature`
- `HexDecode(#[from] hex::FromHexError)`

---

## Step 8 — `kamichain-wallet/src/wallet.rs`

Define:
```rust
pub struct Wallet { signing_key: ed25519_dalek::SigningKey }
```

`Wallet::new()` — `SigningKey::generate(&mut OsRng)`  
`address()` — SHA-256 of the 32-byte public key, return as hex  
`public_key_hex()` — `hex::encode(self.signing_key.verifying_key().to_bytes())`  
`sign_transaction(&self, tx: &mut Transaction)`:
  - Build the message bytes: `format!("{}{}{}{}", tx.sender, tx.recipient, tx.amount, tx.id)` as UTF-8
  - `signing_key.sign(message_bytes)`
  - Store `hex::encode(signature.to_bytes())` in `tx.signature`
  - **Also store** `self.public_key_hex()` in `tx.pub_key` — the mempool uses this to verify ownership

`verify_transaction(tx, public_key_hex)`:
  - Return `Err(MissingSignature)` if `tx.signature.is_none()`
  - Decode `public_key_hex` from hex → 32 raw bytes
  - **Check address ownership**: SHA-256 of those 32 bytes must equal `tx.sender`. If not, return `Err(InvalidPublicKey("pub_key does not match sender address".into()))`. This proves the key owns the address.
  - Build `VerifyingKey` from the decoded bytes
  - Rebuild the same message bytes used during signing
  - Decode signature from hex → `Signature`
  - Call `verifying_key.verify(message, &signature)`, map error

**Also implement:**
- `save_to_file(path)` — hex-encode the 32-byte secret key, write to file
- `Wallet::load_from_file(path)` — read hex, decode back to `SigningKey`

```bash
cargo test -p kamichain-wallet
```

---

## Step 9 — `kamichain-node/src/state.rs`

Define:
```rust
pub type SharedState = Arc<RwLock<NodeState>>;

pub struct NodeState {
    pub chain: Chain,
    pub balances: HashMap<String, u64>,
}
```

`new(difficulty)` — `Chain::new(difficulty)`, empty balances  
`apply_block(block)`:
  - For coinbase tx: `balances[recipient] += amount`
  - For transfer tx: `balances[sender] -= amount`, `balances[recipient] += amount`  
  
`balance_of(address)` — `*self.balances.get(address).unwrap_or(&0)`

```bash
cargo test -p kamichain-node --test state_tests
```

---

## Step 10 — `kamichain-node/src/mempool.rs`

Define:
```rust
pub struct Mempool {
    pending: HashMap<String, Transaction>,
    capacity: usize,
}
```

`new(capacity)` — empty pending map  
`add(tx)` — validate in this order, returning `Err` at first failure:
  1. Reject if `tx.is_coinbase()` — coinbase is created by the miner, not submitted externally
  2. Reject if `tx.sender == tx.recipient` — self-transfer
  3. Reject if `tx.amount == 0` — zero-value transfer
  4. Reject if `tx.pub_key.is_none()` — no public key means unsigned
  5. Call `Wallet::verify_transaction(&tx, pub_key_hex)?` — rejects if the pub_key doesn't own the sender address **or** the signature is invalid
  6. Reject if `pending.contains_key(&tx.id)` — duplicate
  7. Reject if `pending.len() >= capacity` — pool full
  8. Insert into pending map  

`remove(tx_id)` — `pending.remove(tx_id)`  
`take(max)` — sort pending values by `fee` **descending**, return the first `max`. Higher-fee transactions are included in blocks first.  
`len()`, `contains(tx_id)`, `is_empty()` — straightforward

**Note:** `kamichain-node` depends on `kamichain-wallet`, so you can `use kamichain_wallet::Wallet;` inside `mempool.rs`.

```bash
cargo test -p kamichain-node --test mempool_tests
```

---

## Step 11 — `kamichain-node/src/storage.rs`

Define:
```rust
pub struct Storage { path: PathBuf }
```

`new(path)` — store path  
`save_chain(chain)` — serialize chain to JSON, write to `path.with_extension("tmp")`, then `fs::rename` to path (atomic)  
`load_chain()` — read file, deserialize; return `Err` if file missing or corrupt

```bash
cargo test -p kamichain-node --test storage_tests
```

---

## Step 12 — `kamichain-node/src/miner.rs`

Define:
```rust
pub const BLOCK_REWARD: u64 = 50;
pub const MAX_TXS_PER_BLOCK: usize = 100;

pub struct Miner { pub address: String, pub pow: ProofOfWork }
```

`new(address, difficulty)` — store both  
`mine_block(state, mempool)`:
  1. Take up to `MAX_TXS_PER_BLOCK` txs from mempool
  2. Prepend `Transaction::coinbase(&self.address, BLOCK_REWARD)`
  3. Build `Block::new(chain.len() as u64, txs, latest_hash)`
  4. `self.pow.mine(&mut block)`
  5. Return block

`mine_and_commit(state, mempool)`:
  1. `mine_block`
  2. `state.write().chain.add_block(block.clone())`
  3. `state.write().apply_block(&block)`
  4. For each tx in block: `mempool.remove(&tx.id)`
  5. Return block

```bash
cargo test -p kamichain-node --test miner_tests
cargo test -p kamichain-node --test e2e_tests
```

---

## Step 13 — `kamichain-node/src/rpc.rs`

Define:
```rust
pub struct RpcServer {
    listener: std::net::TcpListener,  // bound in new(), converted in run()
    state: SharedState,
    mempool: Arc<Mutex<Mempool>>,
}
pub struct RpcRequest  { pub method: String, pub params: Option<serde_json::Value> }
pub struct RpcResponse { pub ok: bool, pub result: Option<serde_json::Value>, pub error: Option<String> }
```

`new(addr, state, mempool)` — **synchronous**: bind with `std::net::TcpListener::bind(addr)`, call `set_nonblocking(true)`, store it in the struct. Tests call `new` without `.await`.  
`local_port()` — `self.listener.local_addr().unwrap().port()`  
`run(self) -> anyhow::Result<()>` — convert to tokio: `tokio::net::TcpListener::from_std(self.listener)?`, then `loop { accept, tokio::spawn(handle(…)) }`  

`handle` reads one line of JSON, dispatches on `method`, writes one response line:
- `chain_info` → `{ height, latest_hash, difficulty }`
- `chain_block` → look up by index, serialize block or return error
- `tx_submit` → `mempool.add(tx)` or error
- `wallet_balance` → `state.balance_of(address)`
- `node_peers` → return peer list (empty for now until P2P is wired in)
- anything else → `{ ok: false, error: "unknown method" }`

```bash
cargo test -p kamichain-node --test rpc_tests
```

---

## Step 14 — `kamichain-node/src/p2p.rs`

Define:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    NewBlock(Block),
    GetChain,
    Chain(Vec<Block>),
    NewTx(Transaction),
    GetPeers,
    Peers(Vec<String>),
}

pub struct P2PLayer {
    listener: std::net::TcpListener,  // bound in new(), converted in listen()
    peers: Arc<Mutex<Vec<String>>>,
    state: SharedState,
    mempool: Arc<Mutex<Mempool>>,
}
```

The `content = "data"` attribute means the JSON wire format uses a generic `"data"` key for the payload:
```json
{ "type": "new_block", "data": { "index": 5, "hash": "...", ... } }
{ "type": "get_chain" }
{ "type": "chain",    "data": [ { "index": 0, ... }, ... ] }
```
Unit variants (`GetChain`, `GetPeers`) serialize with no content field.

`new(addr, state, mempool)` — **synchronous**: bind with `std::net::TcpListener::bind(addr)`, call `set_nonblocking(true)`. Tests call `new` without `.await`.  
`listen_addr() -> String` — `self.listener.local_addr().unwrap().to_string()`  
`peers() -> Vec<String>` — return a clone of the peer list snapshot  
`listen() -> anyhow::Result<()>` — convert via `tokio::net::TcpListener::from_std`, accept loop, spawn handler per connection  
`connect(peer_addr) -> anyhow::Result<()>` — open TCP connection, add to peer list, spawn handler  
`broadcast_block(block)` — send `Message::NewBlock` to all peers  
`broadcast_tx(tx)` — send `Message::NewTx` to all peers  
`sync_with_peer(addr)` — connect, send `GetChain`, receive `Chain`, call `state.chain.replace`

On receiving `NewBlock`: validate and `chain.add_block`, then `apply_block`. If rejected and peer seems ahead, send `GetChain`.

```bash
cargo test -p kamichain-node --test p2p_tests
```

---

## Step 15 — `kamichain-node/src/bin/node.rs`

Wire everything together:

```
parse --bind, --difficulty, --data-dir, --peer flags
load or create chain from Storage
create SharedState, Mempool (Arc<Mutex>)
connect to --peer addresses via P2PLayer
tokio::spawn RpcServer::run
tokio::spawn P2PLayer::listen
loop { miner.mine_and_commit; p2p.broadcast_block(block) }
```

---

## Step 16 — `kamichain-cli/src/commands/`

Implement each command as a TCP call to the node's RPC server.

`wallet new` — `Wallet::new()`, print address, save to keyfile  
`wallet address --keyfile` — load keyfile, print address  
`wallet balance <addr>` — RPC `wallet_balance`  
`tx send` — load keyfile, create tx, sign, RPC `tx_submit`  
`tx get <id>` — walk chain via `chain_block` calls searching for tx id  
`chain info` — RPC `chain_info`  
`chain block <n>` — RPC `chain_block`  
`chain validate` — RPC `chain_info` + local `is_valid` or a dedicated endpoint  
`node start` — call `kamichain-node` binary or start inline  
`node peers` — RPC `node_peers`  
`node sync <peer>` — RPC call or direct P2P sync

---

## Checkpoints

| Done | Milestone |
|------|-----------|
| ☐ | `cargo test -p kamichain-core` — all pass |
| ☐ | `cargo test -p kamichain-wallet` — all pass |
| ☐ | `cargo test -p kamichain-node --test mempool_tests` |
| ☐ | `cargo test -p kamichain-node --test state_tests` |
| ☐ | `cargo test -p kamichain-node --test storage_tests` |
| ☐ | `cargo test -p kamichain-node --test miner_tests` |
| ☐ | `cargo test -p kamichain-node --test e2e_tests` |
| ☐ | `cargo test -p kamichain-node --test rpc_tests` |
| ☐ | `cargo test -p kamichain-node --test p2p_tests` |
| ☐ | `cargo test --workspace` — everything green |
| ☐ | `cargo bench -p kamichain-core` — benchmarks run |
| ☐ | Two nodes sync over localhost |
| ☐ | `kami wallet new && kami tx send && kami chain info` works end-to-end |
