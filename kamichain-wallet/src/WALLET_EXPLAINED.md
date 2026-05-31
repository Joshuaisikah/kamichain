# wallet.rs — how signing works in kamichain

a wallet is just an ed25519 keypair. the private key stays on disk, the public key becomes your address (via SHA-256), and you use the private key to sign transactions so the network can prove you authorised them without ever seeing your private key.

---

## the imports

```rust
use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::path::Path;
use kamichain_core::Transaction;
use crate::error::WalletError;
```

the ones worth understanding:

| name | what it is |
|------|-----------|
| `SigningKey` | your private key — 32 random bytes, keep this secret |
| `VerifyingKey` | your public key — derived from the private key, safe to share |
| `Signature` | the 64-byte blob produced by signing a message |
| `Signer` | a trait — without this `use`, `.sign()` doesn't exist on `SigningKey` |
| `Verifier` | a trait — without this, `.verify()` doesn't exist on `VerifyingKey` |
| `OsRng` | OS-backed cryptographically secure random number generator |
| `Sha256` / `Digest` | SHA-256 — `Digest` is the trait that gives us `.update()` and `.finalize()` |

the trait imports (`Signer`, `Verifier`, `Digest`) are easy to forget. Rust only makes trait methods available if the trait is in scope. if `.sign()` ever mysteriously doesn't exist, check these imports first.

---

## the struct

```rust
pub struct Wallet {
    signing_key: SigningKey,
}
```

that's the whole thing. one field, no `pub` on it. you can't directly read or overwrite `signing_key` from outside this module — everything goes through the methods. `SigningKey` from `ed25519_dalek` internally holds both the 32-byte private key and the 32-byte public key together, so you can always get the public half with `.verifying_key()`.

---

## new — generating a keypair

```rust
pub fn new() -> Self {
    Self { signing_key: SigningKey::generate(&mut OsRng) }
}
```

`OsRng` asks the operating system for cryptographically random bytes. `generate(&mut OsRng)` needs `&mut` because each call consumes entropy from the generator — Rust requires `&mut` whenever something changes. every call to `Wallet::new()` produces a completely different keypair.

`Self` (capital S) is just an alias for the type being implemented — here it means `Wallet`. using `Self` instead of `Wallet` is cleaner when refactoring.

---

## address — your public identifier

```rust
pub fn address(&self) -> String {
    let pub_key_bytes = self.signing_key.verifying_key().to_bytes();
    let mut hasher = Sha256::new();
    hasher.update(pub_key_bytes);
    format!("{:x}", hasher.finalize())
}
```

your address is SHA-256 of your public key, not the key itself. same approach bitcoin uses. the steps:

1. `.verifying_key()` — extract the public half (32 bytes)
2. `.to_bytes()` — get the raw `[u8; 32]` array
3. hash it with SHA-256
4. `format!("{:x}", ...)` — format as lowercase hex → 64 chars

**why hash the public key instead of using it directly?** two reasons. first, 64 hex chars is cleaner to display than raw bytes. second, even if ed25519 were ever broken, an attacker would also need to break SHA-256 to work backwards from address to key. defense in depth.

---

## public_key_hex — sharing your public key

```rust
pub fn public_key_hex(&self) -> String {
    hex::encode(self.signing_key.verifying_key().to_bytes())
}
```

`hex::encode` turns a byte slice into a lowercase hex string. 32 bytes → 64 hex chars. this gets stored on the transaction when you sign it, so verifiers know which public key to check the signature against.

---

## sign_transaction — attaching your signature

```rust
pub fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), WalletError> {
    let message = format!("{}{}{}{}", tx.sender, tx.recipient, tx.amount, tx.id);
    let signature = self.signing_key.sign(message.as_bytes());
    tx.signature = Some(hex::encode(signature.to_bytes()));
    tx.pub_key   = Some(self.public_key_hex());
    Ok(())
}
```

**what gets signed:** `sender + recipient + amount + id` concatenated into one string, then signed as bytes. the signature mathematically locks all four fields — change any one of them after signing and the signature stops verifying.

`tx.id` already includes the nonce (it's `SHA-256(sender + recipient + amount + nonce)`), so the nonce is covered indirectly.

**one thing to note:** `fee` is not in the signed message. that means fee can technically be changed after signing without breaking verification. the mempool currently checks `amount + fee <= balance` at submission time, so in practice this is fine — but it's something to fix properly if this ever becomes a production system.

`tx.signature` and `tx.pub_key` are both `Option<String>` — they start as `None` when the transaction is created and get set to `Some(...)` here. `Ok(())` means "succeeded, nothing to return" — `()` is Rust's unit type (like `void`).

**why `&mut Transaction`?** we need to write to `tx.signature` and `tx.pub_key`. Rust requires `&mut` for any mutation. if the parameter were `&Transaction` (no `mut`) the compiler would reject the field assignments.

---

## verify_transaction — checking someone's signature

```rust
pub fn verify_transaction(tx: &Transaction, pub_key_hex: &str) -> Result<bool, WalletError> {
```

no `&self` — this is an associated function (like a static method). you call it as `Wallet::verify_transaction(&tx, key_hex)`. it doesn't need a wallet instance, just the transaction and the public key you want to verify against.

step by step:

```rust
let sig_hex = tx.signature.as_ref().ok_or(WalletError::MissingSignature)?;
```
`.as_ref()` borrows the `String` inside the `Option` without consuming it. `.ok_or(err)` converts `Option<T>` → `Result<T, E>` — `None` becomes `Err(MissingSignature)`. `?` returns that error immediately if it's `Err`.

```rust
let pub_key_bytes = hex::decode(pub_key_hex)?;
let pub_key_array: [u8; 32] = pub_key_bytes.try_into()
    .map_err(|_| WalletError::InvalidPublicKey("Key must be 32 bytes".into()))?;
```
decode the hex string back to bytes, then convert from `Vec<u8>` to `[u8; 32]`. `.try_into()` only succeeds if the vec has exactly 32 bytes. `.map_err(|_| ...)` swaps the error type for our own — the `|_|` closure ignores the original error value.

```rust
let mut hasher = Sha256::new();
hasher.update(pub_key_array);
let derived_address = format!("{:x}", hasher.finalize());
if derived_address != tx.sender {
    return Err(WalletError::InvalidPublicKey("Public key does not match sender address".into()));
}
```
re-derive the address from the provided public key (same formula as `address()`) and compare it to `tx.sender`. if they don't match, the key belongs to someone else — reject. this is the core identity check.

```rust
let verifying_key = VerifyingKey::from_bytes(&pub_key_array)?;
let message = format!("{}{}{}{}", tx.sender, tx.recipient, tx.amount, tx.id);
```
build the `VerifyingKey` object, then reconstruct the **exact same string** that was signed. sign and verify must hash identical bytes — if even one character differs, verification fails.

```rust
let sig_bytes = hex::decode(sig_hex)?;
let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| WalletError::VerificationFailed)?;
let signature = Signature::from_bytes(&sig_array);
verifying_key.verify(message.as_bytes(), &signature)
    .map_err(|_| WalletError::VerificationFailed)?;
Ok(true)
```
decode the stored signature hex → bytes → fixed 64-byte array → `Signature`. ed25519 signatures are always 64 bytes. `.verify()` returns `Ok(())` if valid, `Err` if not. map the error to `VerificationFailed`, propagate with `?`, return `Ok(true)` if we make it through.

---

## save_to_file / load_from_file — persisting the key

```rust
pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), WalletError> {
    let hex = hex::encode(self.signing_key.to_bytes());
    std::fs::write(path, hex).map_err(|e| WalletError::InvalidPublicKey(e.to_string()))
}
```

`signing_key.to_bytes()` extracts the raw 32-byte private key. we hex-encode it and write as plain text. **this file is your private key** — whoever has it owns the wallet.

`impl AsRef<Path>` means "any type that can be treated as a path" — `&str`, `String`, `PathBuf`, all work without the caller needing to convert.

```rust
pub fn load_from_file(path: impl AsRef<Path>) -> Result<Wallet, WalletError> {
    let hex = std::fs::read_to_string(path)
        .map_err(|e| WalletError::InvalidPublicKey(e.to_string()))?;
    let bytes = hex::decode(hex.trim())?;
    let array: [u8; 32] = bytes.try_into()
        .map_err(|_| WalletError::InvalidPublicKey("Key must be 32 bytes".into()))?;
    let signing_key = SigningKey::from_bytes(&array);
    Ok(Wallet { signing_key })
}
```

exact reverse. `.trim()` strips the trailing newline that text editors usually add — without it `hex::decode` would fail on valid files.

---

## error.rs

```rust
pub enum WalletError {
    InvalidPublicKey(String),   // bad key bytes, wrong key for sender, IO errors
    VerificationFailed,         // signature didn't check out
    MissingSignature,           // tx.signature is None
    HexDecode(#[from] hex::FromHexError),  // invalid hex strings
}
```

`#[from] hex::FromHexError` means any `hex::FromHexError` automatically converts into `WalletError::HexDecode`. that's why `hex::decode(...)?` works even though the function returns `Result<_, WalletError>` — the `?` operator calls `.into()` on the error and the `From` impl handles the conversion.

`InvalidPublicKey` is doing a bit too much — it covers bad key formats, mismatched keys, AND file IO errors. a more precise codebase would split those. fine for now.

---

## key rust concepts used here

### Option and Result

```rust
Option<String>       // Some("abc") or None
Result<bool, Err>    // Ok(true)   or Err(WalletError::...)
```

`Option` is for values that might not exist. `Result` is for operations that can fail. the `?` operator is how you work with `Result` without writing `match` everywhere — it unwraps `Ok(value)` or returns `Err(e)` immediately from the current function.

### references — & and &mut

```rust
fn address(&self)                          // borrow self, read only
fn sign_transaction(&self, tx: &mut Transaction)  // borrow tx, read+write
```

`&T` is a shared borrow — many readers, no writers. `&mut T` is an exclusive borrow — one writer, no other readers. Rust enforces this at compile time. if you forget `mut` and try to write to a field, the compiler tells you exactly which line.

### closures

```rust
.map_err(|_| WalletError::VerificationFailed)
.map_err(|e| WalletError::InvalidPublicKey(e.to_string()))
```

`|_|` is an anonymous function that takes one argument and ignores it. `|e|` takes the argument and uses it. `.map_err` transforms the error type inside a `Result` without touching the `Ok` value.

### method chaining

```rust
tx.signature.as_ref().ok_or(WalletError::MissingSignature)?
```

each method transforms the type, producing the input for the next call. reading left to right: `Option<String>` → `Option<&String>` → `Result<&String, WalletError>` → `&String` (or early return).

### traits

`Signer`, `Verifier`, `Digest`, `AsRef` — these are interfaces. a type can implement multiple traits. `impl AsRef<Path>` in function signatures means "any type that implements `AsRef<Path>`" — it's how Rust avoids needing overloads.

---

## the crypto in plain terms

**ed25519** — an elliptic curve signature algorithm. private key is 32 random bytes. public key is derived from it deterministically (you can always recompute the public key from the private key, never the reverse). signing produces a 64-byte signature. verifying takes public key + message + signature and returns yes/no. the math guarantees that producing a valid signature without the private key is computationally infeasible.

**why SHA-256 the public key for the address?** the public key is 32 bytes of binary data. SHA-256 gives a clean 64-char hex string that looks like a normal blockchain address. it also means the raw public key isn't exposed in the address, which gives a small extra layer of security.

**what the signature actually proves:** that at the time of signing, the signer knew the private key corresponding to `tx.sender`'s address, and they signed exactly those transaction fields. the network can verify this with just the public key — no private key needed.
