use kamichain_core::Transaction;
use kamichain_wallet::Wallet;

#[test]
fn new_wallet_generates_a_non_empty_address() {
    let wallet = Wallet::new();
    assert!(!wallet.address().is_empty());
}

#[test]
fn two_wallets_have_different_addresses() {
    let a = Wallet::new();
    let b = Wallet::new();
    assert_ne!(a.address(), b.address());
}

#[test]
fn address_is_valid_hex() {
    let wallet = Wallet::new();
    let addr = wallet.address();
    assert!(addr.chars().all(|c| c.is_ascii_hexdigit()), "address: {}", addr);
}

#[test]
fn signing_a_transaction_sets_signature() {
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 10);
    wallet.sign_transaction(&mut tx).expect("signing failed");
    assert!(tx.signature.is_some());
}

#[test]
fn signature_is_valid_hex() {
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 10);
    wallet.sign_transaction(&mut tx).expect("signing failed");
    let sig = tx.signature.as_ref().unwrap();
    assert!(sig.chars().all(|c| c.is_ascii_hexdigit()), "sig: {}", sig);
}

#[test]
fn verification_passes_with_correct_public_key() {
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 10);
    wallet.sign_transaction(&mut tx).expect("signing failed");

    let result = Wallet::verify_transaction(&tx, &wallet.public_key_hex());
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn verification_fails_with_wrong_public_key() {
    let signer = Wallet::new();
    let other = Wallet::new();

    let mut tx = Transaction::new(signer.address(), "bob", 10);
    signer.sign_transaction(&mut tx).expect("signing failed");

    let result = Wallet::verify_transaction(&tx, &other.public_key_hex());
    // Either returns Ok(false) or Err — both are acceptable as "failed verification"
    match result {
        Ok(valid) => assert!(!valid),
        Err(_) => {} // also fine
    }
}

#[test]
fn verification_fails_on_unsigned_transaction() {
    let wallet = Wallet::new();
    let tx = Transaction::new(wallet.address(), "bob", 10);
    let result = Wallet::verify_transaction(&tx, &wallet.public_key_hex());
    assert!(result.is_err());
}

#[test]
fn tampered_transaction_fails_verification() {
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 10);
    wallet.sign_transaction(&mut tx).expect("signing failed");

    // Tamper with amount after signing
    tx.amount = 9999;

    let result = Wallet::verify_transaction(&tx, &wallet.public_key_hex());
    match result {
        Ok(valid) => assert!(!valid),
        Err(_) => {}
    }
}

#[test]
fn two_signatures_of_same_data_are_deterministic_or_both_valid() {
    // ed25519 is deterministic — same key + same message = same signature
    let wallet = Wallet::new();
    let mut tx1 = Transaction::new(wallet.address(), "bob", 10);
    let mut tx2 = Transaction::new(wallet.address(), "bob", 10);

    wallet.sign_transaction(&mut tx1).unwrap();
    wallet.sign_transaction(&mut tx2).unwrap();

    // If deterministic: signatures match
    // Either way, both must verify
    assert!(Wallet::verify_transaction(&tx1, &wallet.public_key_hex()).unwrap());
    assert!(Wallet::verify_transaction(&tx2, &wallet.public_key_hex()).unwrap());
}
