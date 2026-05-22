# kamichain-wallet

Keypair generation and transaction signing using ed25519.

## What to build

### `src/error.rs`
`WalletError` covering: invalid public key hex, missing signature, failed signature verification, hex decode errors.

### `src/wallet.rs`
`Wallet` wraps an `ed25519_dalek::SigningKey`.

- `Wallet::new()` — generate a random keypair using `rand::rngs::OsRng`
- `address()` — hex-encoded SHA-256 of the public key (first 20 bytes for a short address, or full 32)
- `public_key_hex()` — full 32-byte public key as hex
- `sign_transaction(&self, tx: &mut Transaction)` — serialize the tx fields (excluding signature), sign with the private key, store the signature as hex in `tx.signature`
- `Wallet::verify_transaction(tx, public_key_hex)` — decode the public key, re-serialize the tx fields, verify the signature

The serialization for signing must be consistent: the same fields hashed in the same order every time, and the signature field itself must be excluded from the signed data.

## Tests

```bash
cargo test -p kamichain-wallet
```
