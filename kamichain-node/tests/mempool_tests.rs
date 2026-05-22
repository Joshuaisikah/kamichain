use kamichain_core::Transaction;
use kamichain_node::mempool::Mempool;

#[test]
fn new_mempool_is_empty() {
    let pool = Mempool::new(100);
    assert!(pool.is_empty());
    assert_eq!(pool.len(), 0);
}

#[test]
fn adding_a_transaction_increases_len() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "bob", 10);
    pool.add(tx).unwrap();
    assert_eq!(pool.len(), 1);
}

#[test]
fn duplicate_transaction_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "bob", 10);
    pool.add(tx.clone()).unwrap();
    assert!(pool.add(tx).is_err());
}

#[test]
fn pool_rejects_when_full() {
    let mut pool = Mempool::new(2);
    pool.add(Transaction::new("alice", "bob", 1)).unwrap();
    pool.add(Transaction::new("alice", "bob", 2)).unwrap();
    let result = pool.add(Transaction::new("alice", "bob", 3));
    assert!(result.is_err());
}

#[test]
fn contains_returns_true_for_added_tx() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "bob", 10);
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
    let tx = Transaction::new("alice", "bob", 10);
    let id = tx.id.clone();
    pool.add(tx).unwrap();
    pool.remove(&id);
    assert_eq!(pool.len(), 0);
}

#[test]
fn remove_nonexistent_is_a_noop() {
    let mut pool = Mempool::new(100);
    pool.remove("ghost"); // should not panic
    assert_eq!(pool.len(), 0);
}

#[test]
fn take_returns_up_to_max_transactions() {
    let mut pool = Mempool::new(100);
    for i in 0..10 {
        pool.add(Transaction::new("alice", "bob", i as u64)).unwrap();
    }
    let taken = pool.take(5);
    assert_eq!(taken.len(), 5);
}

#[test]
fn take_returns_all_when_fewer_than_max() {
    let mut pool = Mempool::new(100);
    pool.add(Transaction::new("alice", "bob", 1)).unwrap();
    pool.add(Transaction::new("alice", "bob", 2)).unwrap();
    let taken = pool.take(50);
    assert_eq!(taken.len(), 2);
}

#[test]
fn take_does_not_remove_transactions_from_pool() {
    let mut pool = Mempool::new(100);
    pool.add(Transaction::new("alice", "bob", 1)).unwrap();
    let _ = pool.take(10);
    assert_eq!(pool.len(), 1);
}

// ── Balance validation tests ──────────────────────────────────
// The mempool can optionally check balances at admission time.
// If it does, these tests specify the expected behaviour.
// If balance checking is done at mining time instead, these
// tests still document the invariant that must hold.

#[test]
fn transfer_with_zero_amount_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "bob", 0);
    assert!(pool.add(tx).is_err(), "zero-amount transfer should be rejected");
}

#[test]
fn coinbase_transaction_is_not_admitted_to_mempool() {
    // Coinbase txs are created by the miner, not submitted externally
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
