use kamichain_core::Transaction;
use kamichain_node::mempool::Mempool;
use kamichain_wallet::Wallet;

/// Create a signed Transfer transaction from a fresh wallet.
/// Each call produces a unique sender (new keypair) so IDs never collide.
fn signed_tx(amount: u64) -> Transaction {
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "recipient", amount);
    wallet.sign_transaction(&mut tx).unwrap();
    tx
}

// ── Basic mechanics ───────────────────────────────────────────

#[test]
fn new_mempool_is_empty() {
    let pool = Mempool::new(100);
    assert!(pool.is_empty());
    assert_eq!(pool.len(), 0);
}

#[test]
fn adding_a_transaction_increases_len() {
    let mut pool = Mempool::new(100);
    pool.add(signed_tx(10)).unwrap();
    assert_eq!(pool.len(), 1);
}

#[test]
fn duplicate_transaction_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = signed_tx(10);
    pool.add(tx.clone()).unwrap();
    assert!(pool.add(tx).is_err());
}

#[test]
fn pool_rejects_when_full() {
    let mut pool = Mempool::new(2);
    pool.add(signed_tx(1)).unwrap();
    pool.add(signed_tx(2)).unwrap();
    assert!(pool.add(signed_tx(3)).is_err());
}

#[test]
fn contains_returns_true_for_added_tx() {
    let mut pool = Mempool::new(100);
    let tx = signed_tx(10);
    let id = tx.id.clone();
    pool.add(tx).unwrap();
    assert!(pool.contains(&id));
}

#[test]
fn contains_returns_false_for_unknown_tx() {
    let pool = Mempool::new(100);
    assert!(!pool.contains("not_a_real_id"));
}

#[test]
fn remove_decreases_len() {
    let mut pool = Mempool::new(100);
    let tx = signed_tx(10);
    let id = tx.id.clone();
    pool.add(tx).unwrap();
    pool.remove(&id);
    assert_eq!(pool.len(), 0);
}

#[test]
fn remove_nonexistent_is_a_noop() {
    let mut pool = Mempool::new(100);
    pool.remove("ghost");
    assert_eq!(pool.len(), 0);
}

#[test]
fn take_returns_up_to_max_transactions() {
    let mut pool = Mempool::new(100);
    for i in 1..=10 {
        pool.add(signed_tx(i)).unwrap();
    }
    let taken = pool.take(5);
    assert_eq!(taken.len(), 5);
}

#[test]
fn take_returns_all_when_fewer_than_max() {
    let mut pool = Mempool::new(100);
    pool.add(signed_tx(1)).unwrap();
    pool.add(signed_tx(2)).unwrap();
    let taken = pool.take(50);
    assert_eq!(taken.len(), 2);
}

#[test]
fn take_does_not_remove_transactions_from_pool() {
    let mut pool = Mempool::new(100);
    pool.add(signed_tx(1)).unwrap();
    let _ = pool.take(10);
    assert_eq!(pool.len(), 1);
}

// ── Structural validation ─────────────────────────────────────

#[test]
fn transfer_with_zero_amount_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "bob", 0);
    assert!(pool.add(tx).is_err(), "zero-amount transfer should be rejected");
}

#[test]
fn coinbase_transaction_is_not_admitted_to_mempool() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::coinbase("miner", 50);
    assert!(pool.add(tx).is_err(), "coinbase should not be user-submittable");
}

#[test]
fn transaction_with_sender_equal_to_recipient_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "alice", 10);
    assert!(pool.add(tx).is_err(), "self-transfer should be rejected");
}

// ── Signature verification ────────────────────────────────────

#[test]
fn unsigned_transfer_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice_addr", "bob_addr", 10);
    assert!(pool.add(tx).is_err(), "unsigned transfer must be rejected");
}

#[test]
fn signed_transfer_is_accepted() {
    let mut pool = Mempool::new(100);
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob_addr", 10);
    wallet.sign_transaction(&mut tx).unwrap();
    assert!(pool.add(tx).is_ok());
}

#[test]
fn transfer_signed_by_wrong_key_is_rejected() {
    let mut pool  = Mempool::new(100);
    let wallet_a  = Wallet::new();
    let wallet_b  = Wallet::new();
    // Claims to be from wallet_a's address, but pub_key will be wallet_b's
    let mut tx = Transaction::new(wallet_a.address(), "bob_addr", 10);
    wallet_b.sign_transaction(&mut tx).unwrap();
    assert!(pool.add(tx).is_err(), "mismatched pub_key/sender must be rejected");
}

// ── Fee priority ──────────────────────────────────────────────

#[test]
fn transaction_has_zero_fee_by_default() {
    let tx = Transaction::new("alice", "bob", 10);
    assert_eq!(tx.fee, 0);
}

#[test]
fn take_returns_highest_fee_transactions_first() {
    let mut pool = Mempool::new(100);

    let mut tx_low  = signed_tx(1); tx_low.fee  = 1;
    let mut tx_high = signed_tx(2); tx_high.fee = 10;
    let mut tx_mid  = signed_tx(3); tx_mid.fee  = 5;

    pool.add(tx_low).unwrap();
    pool.add(tx_high).unwrap();
    pool.add(tx_mid).unwrap();

    let taken = pool.take(3);
    assert_eq!(taken[0].fee, 10);
    assert_eq!(taken[1].fee, 5);
    assert_eq!(taken[2].fee, 1);
}

#[test]
fn take_selects_highest_fee_when_pool_exceeds_max() {
    let mut pool = Mempool::new(100);

    // 5 txs with fees [1, 8, 3, 10, 2]; take top 3 → fees [10, 8, 3]
    for (i, &fee) in [1u64, 8, 3, 10, 2].iter().enumerate() {
        let mut tx = signed_tx(i as u64 + 1);
        tx.fee = fee;
        pool.add(tx).unwrap();
    }

    let taken      = pool.take(3);
    let taken_fees: Vec<u64> = taken.iter().map(|t| t.fee).collect();
    assert_eq!(taken_fees, vec![10, 8, 3]);
}
