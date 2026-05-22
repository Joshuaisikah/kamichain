use kamichain_core::{Transaction, TxType};

#[test]
fn coinbase_has_correct_type() {
    let tx = Transaction::coinbase("miner_addr", 50);
    assert_eq!(tx.tx_type, TxType::Coinbase);
}

#[test]
fn coinbase_sender_is_empty() {
    let tx = Transaction::coinbase("miner_addr", 50);
    assert!(tx.sender.is_empty());
}

#[test]
fn coinbase_amount_is_set() {
    let tx = Transaction::coinbase("miner_addr", 50);
    assert_eq!(tx.amount, 50);
}

#[test]
fn coinbase_is_coinbase_returns_true() {
    let tx = Transaction::coinbase("miner_addr", 50);
    assert!(tx.is_coinbase());
}

#[test]
fn transfer_is_coinbase_returns_false() {
    let tx = Transaction::new("alice", "bob", 10);
    assert!(!tx.is_coinbase());
}

#[test]
fn transfer_has_correct_type() {
    let tx = Transaction::new("alice", "bob", 10);
    assert_eq!(tx.tx_type, TxType::Transfer);
}

#[test]
fn transfer_stores_sender_and_recipient() {
    let tx = Transaction::new("alice", "bob", 42);
    assert_eq!(tx.sender, "alice");
    assert_eq!(tx.recipient, "bob");
    assert_eq!(tx.amount, 42);
}

#[test]
fn transaction_id_is_not_empty() {
    let tx = Transaction::new("alice", "bob", 10);
    assert!(!tx.id.is_empty());
}

#[test]
fn same_transaction_produces_same_id() {
    let tx = Transaction::new("alice", "bob", 10);
    assert_eq!(tx.id, tx.compute_id());
}

#[test]
fn different_transactions_produce_different_ids() {
    let tx1 = Transaction::new("alice", "bob", 10);
    let tx2 = Transaction::new("alice", "bob", 99);
    assert_ne!(tx1.id, tx2.id);
}

#[test]
fn new_transaction_has_no_signature() {
    let tx = Transaction::new("alice", "bob", 10);
    assert!(tx.signature.is_none());
}

#[test]
fn transaction_has_zero_fee_by_default() {
    let tx = Transaction::new("alice", "bob", 10);
    assert_eq!(tx.fee, 0);
}

#[test]
fn transaction_pub_key_is_none_before_signing() {
    let tx = Transaction::new("alice", "bob", 10);
    assert!(tx.pub_key.is_none());
}
