# Protocol Specification

## RPC Protocol

The RPC server listens on TCP. Each connection carries exactly one request and receives exactly one response, then the connection closes.

Messages are newline-terminated JSON (`\n`).

### Request format

```json
{ "method": "<name>", "params": { ... } }
```

`params` is omitted for methods that take no arguments.

### Response format

```json
{ "ok": true,  "result": { ... } }
{ "ok": false, "error": "<human-readable message>" }
```

### Methods

#### `chain_info`
No params. Returns chain height, latest block hash, and mining difficulty.

```json
// request
{ "method": "chain_info" }

// response
{ "ok": true, "result": { "height": 42, "latest_hash": "00003f...", "difficulty": 4 } }
```

#### `chain_block`
Returns full block data for the given index.

```json
// request
{ "method": "chain_block", "params": { "index": 5 } }

// response
{ "ok": true, "result": { "index": 5, "hash": "...", "prev_hash": "...", "nonce": 84231, "timestamp": 1716000000, "transactions": [...] } }

// error (block not found)
{ "ok": false, "error": "block 5 not found" }
```

#### `tx_submit`
Submit a signed transaction to the mempool.

```json
// request
{ "method": "tx_submit", "params": { "tx": { "id": "...", "tx_type": "Transfer", "sender": "...", "recipient": "...", "amount": 10, "signature": "..." } } }

// response
{ "ok": true, "result": { "tx_id": "..." } }

// error
{ "ok": false, "error": "transaction already in mempool" }
```

#### `wallet_balance`
Return the confirmed balance for an address.

```json
// request
{ "method": "wallet_balance", "params": { "address": "a1b2c3..." } }

// response
{ "ok": true, "result": { "address": "a1b2c3...", "balance": 150 } }
```

#### `node_peers`
Return connected peer addresses.

```json
// request
{ "method": "node_peers" }

// response
{ "ok": true, "result": { "peers": ["192.168.1.2:8332", "10.0.0.5:8332"] } }
```

#### Unknown method

```json
{ "ok": false, "error": "unknown method: foo" }
```

---

## P2P Protocol

Peers communicate over persistent TCP connections. Messages are newline-terminated JSON.

### Message types

#### `new_block`
Broadcast when a node mines a new block. Recipients validate and add it to their chain.

```json
{ "type": "new_block", "block": { "index": 10, "hash": "...", ... } }
```

#### `get_chain`
Request the full chain from a peer (used during initial sync or after a fork is detected).

```json
{ "type": "get_chain" }
```

#### `chain`
Response to `get_chain`. Contains the sender's full block list.

```json
{ "type": "chain", "blocks": [ { "index": 0, ... }, { "index": 1, ... }, ... ] }
```

#### `new_tx`
Broadcast a new unconfirmed transaction to all peers.

```json
{ "type": "new_tx", "tx": { "id": "...", "sender": "...", "recipient": "...", "amount": 5, "signature": "..." } }
```

#### `get_peers`
Request the peer list from a connected node (peer exchange).

```json
{ "type": "get_peers" }
```

#### `peers`
Response to `get_peers`.

```json
{ "type": "peers", "addrs": ["192.168.1.3:8332", "10.0.0.9:8332"] }
```

---

## Connection lifecycle

```
A connects to B (TCP)
    │
    ▼
A sends get_chain               ← initial sync
B responds with chain
A calls Chain::replace if longer
    │
    ▼
Both nodes enter message loop:
    - on new_block received → validate → add or request full chain
    - on new_tx received   → add to mempool if not already present
    - on get_peers         → respond with known peer list
    │
    ▼
Either side closes TCP → connection dropped, removed from peer list
```

---

## Transaction signing

The bytes signed by the wallet are the SHA-256 of:

```
sender_hex || recipient_hex || amount_as_le_u64_bytes || tx_id_hex
```

The signature is stored as lowercase hex in `Transaction.signature`.

Verification: re-derive the same byte sequence, decode the hex public key into an `ed25519_dalek::VerifyingKey`, and call `verify`.
