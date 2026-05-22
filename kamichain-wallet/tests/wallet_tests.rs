use kamichain_core::Transaction;
use kamichain_wallet::Wallet;
use std::path::PathBuf;

fn tmp_keyfile() -> PathBuf {
    std::env::temp_dir().join(format!(
        "kami_test_key_{}.hex",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ))
}

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
    let wallet = Wallet::new();
    let mut tx1 = Transaction::new(wallet.address(), "bob", 10);
    let mut tx2 = Transaction::new(wallet.address(), "bob", 10);

    wallet.sign_transaction(&mut tx1).unwrap();
    wallet.sign_transaction(&mut tx2).unwrap();

    assert!(Wallet::verify_transaction(&tx1, &wallet.public_key_hex()).unwrap());
    assert!(Wallet::verify_transaction(&tx2, &wallet.public_key_hex()).unwrap());
}

// ── Keyfile tests ─────────────────────────────────────────────

#[test]
fn save_and_load_keyfile_restores_same_address() {
    let path   = tmp_keyfile();
    let wallet = Wallet::new();
    let addr   = wallet.address();

    wallet.save_to_file(&path).unwrap();
    let loaded = Wallet::load_from_file(&path).unwrap();

    assert_eq!(loaded.address(), addr);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_and_load_keyfile_restores_same_public_key() {
    let path   = tmp_keyfile();
    let wallet = Wallet::new();

    wallet.save_to_file(&path).unwrap();
    let loaded = Wallet::load_from_file(&path).unwrap();

    assert_eq!(loaded.public_key_hex(), wallet.public_key_hex());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loaded_wallet_can_sign_and_verify() {
    let path   = tmp_keyfile();
    let wallet = Wallet::new();
    wallet.save_to_file(&path).unwrap();

    let loaded    = Wallet::load_from_file(&path).unwrap();
    let mut tx    = Transaction::new(loaded.address(), "carol", 25);
    loaded.sign_transaction(&mut tx).unwrap();

    assert!(Wallet::verify_transaction(&tx, &loaded.public_key_hex()).unwrap());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_from_missing_file_returns_error() {
    let result = Wallet::load_from_file("/tmp/kamichain_no_such_key_xyz.hex");
    assert!(result.is_err());
}

#[test]
fn two_wallets_saved_to_different_files_load_independently() {
    let path_a = tmp_keyfile();
    let path_b = tmp_keyfile();

    let wallet_a = Wallet::new();
    let wallet_b = Wallet::new();
    wallet_a.save_to_file(&path_a).unwrap();
    wallet_b.save_to_file(&path_b).unwrap();

    let loaded_a = Wallet::load_from_file(&path_a).unwrap();
    let loaded_b = Wallet::load_from_file(&path_b).unwrap();
    assert_ne!(loaded_a.address(), loaded_b.address());

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}
