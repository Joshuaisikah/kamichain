use kamichain_core::Transaction;
use kamichain_node::mempool::Mempool;
use kamichain_wallet::Wallet;

/// Build a signed transfer from a fresh wallet with enough on-chain balance.
/// Each call produces a unique sender (new keypair) so IDs never collide.
fn signed_tx(amount: u64) -> Transaction {
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "recipient", amount, 0);
    wallet.sign_transaction(&mut tx).unwrap();
    tx
}

/// Generous balance used for tests that don't care about the balance check.
const ENOUGH: u64 = 10_000;

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
    pool.add(signed_tx(10), ENOUGH).unwrap();
    assert_eq!(pool.len(), 1);
}

#[test]
fn duplicate_transaction_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = signed_tx(10);
    pool.add(tx.clone(), ENOUGH).unwrap();
    assert!(pool.add(tx, ENOUGH).is_err());
}

#[test]
fn pool_rejects_when_full() {
    let mut pool = Mempool::new(2);
    pool.add(signed_tx(1), ENOUGH).unwrap();
    pool.add(signed_tx(2), ENOUGH).unwrap();
    assert!(pool.add(signed_tx(3), ENOUGH).is_err());
}

#[test]
fn contains_returns_true_for_added_tx() {
    let mut pool = Mempool::new(100);
    let tx = signed_tx(10);
    let id = tx.id.clone();
    pool.add(tx, ENOUGH).unwrap();
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
    pool.add(tx, ENOUGH).unwrap();
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
        pool.add(signed_tx(i), ENOUGH).unwrap();
    }
    let taken = pool.take(5);
    assert_eq!(taken.len(), 5);
}

#[test]
fn take_returns_all_when_fewer_than_max() {
    let mut pool = Mempool::new(100);
    pool.add(signed_tx(1), ENOUGH).unwrap();
    pool.add(signed_tx(2), ENOUGH).unwrap();
    let taken = pool.take(50);
    assert_eq!(taken.len(), 2);
}

#[test]
fn take_does_not_remove_transactions_from_pool() {
    let mut pool = Mempool::new(100);
    pool.add(signed_tx(1), ENOUGH).unwrap();
    let _ = pool.take(10);
    assert_eq!(pool.len(), 1);
}

// ── Structural validation ─────────────────────────────────────

#[test]
fn transfer_with_zero_amount_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "bob", 0, 0);
    assert!(pool.add(tx, ENOUGH).is_err(), "zero-amount transfer should be rejected");
}

#[test]
fn coinbase_transaction_is_not_admitted_to_mempool() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::coinbase("miner", 50);
    assert!(pool.add(tx, ENOUGH).is_err(), "coinbase should not be user-submittable");
}

#[test]
fn transaction_with_sender_equal_to_recipient_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice", "alice", 10, 0);
    assert!(pool.add(tx, ENOUGH).is_err(), "self-transfer should be rejected");
}

// ── Signature verification ────────────────────────────────────

#[test]
fn unsigned_transfer_is_rejected() {
    let mut pool = Mempool::new(100);
    let tx = Transaction::new("alice_addr", "bob_addr", 10, 0);
    assert!(pool.add(tx, ENOUGH).is_err(), "unsigned transfer must be rejected");
}

#[test]
fn signed_transfer_is_accepted() {
    let mut pool = Mempool::new(100);
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob_addr", 10, 0);
    wallet.sign_transaction(&mut tx).unwrap();
    assert!(pool.add(tx, ENOUGH).is_ok());
}

#[test]
fn transfer_signed_by_wrong_key_is_rejected() {
    let mut pool  = Mempool::new(100);
    let wallet_a  = Wallet::new();
    let wallet_b  = Wallet::new();
    let mut tx = Transaction::new(wallet_a.address(), "bob_addr", 10, 0);
    wallet_b.sign_transaction(&mut tx).unwrap();
    assert!(pool.add(tx, ENOUGH).is_err(), "mismatched pub_key/sender must be rejected");
}

// ── Balance validation ────────────────────────────────────────

#[test]
fn transaction_rejected_when_amount_exceeds_balance() {
    let mut pool = Mempool::new(100);
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 100, 0);
    wallet.sign_transaction(&mut tx).unwrap();
    assert!(pool.add(tx, 50).is_err(), "amount > balance must be rejected");
}

#[test]
fn transaction_rejected_when_fee_pushes_total_over_balance() {
    let mut pool = Mempool::new(100);
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 80, 30);
    wallet.sign_transaction(&mut tx).unwrap();
    // amount(80) + fee(30) = 110 > balance(100)
    assert!(pool.add(tx, 100).is_err(), "amount + fee > balance must be rejected");
}

#[test]
fn transaction_accepted_when_amount_plus_fee_equals_balance() {
    let mut pool = Mempool::new(100);
    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 70, 30);
    wallet.sign_transaction(&mut tx).unwrap();
    // amount(70) + fee(30) = 100 == balance(100)
    assert!(pool.add(tx, 100).is_ok(), "exact balance should be accepted");
}

// ── Fee priority ──────────────────────────────────────────────

#[test]
fn transaction_has_zero_fee_by_default() {
    let tx = Transaction::new("alice", "bob", 10, 0);
    assert_eq!(tx.fee, 0);
}

#[test]
fn transaction_fee_is_set_at_construction() {
    let tx = Transaction::new("alice", "bob", 10, 5);
    assert_eq!(tx.fee, 5);
}

#[test]
fn take_returns_highest_fee_transactions_first() {
    let mut pool = Mempool::new(100);

    let wallet_low  = Wallet::new();
    let wallet_high = Wallet::new();
    let wallet_mid  = Wallet::new();

    let mut tx_low  = Transaction::new(wallet_low.address(),  "r", 1, 1);
    let mut tx_high = Transaction::new(wallet_high.address(), "r", 2, 10);
    let mut tx_mid  = Transaction::new(wallet_mid.address(),  "r", 3, 5);

    wallet_low.sign_transaction(&mut tx_low).unwrap();
    wallet_high.sign_transaction(&mut tx_high).unwrap();
    wallet_mid.sign_transaction(&mut tx_mid).unwrap();

    pool.add(tx_low,  ENOUGH).unwrap();
    pool.add(tx_high, ENOUGH).unwrap();
    pool.add(tx_mid,  ENOUGH).unwrap();

    let taken = pool.take(3);
    assert_eq!(taken[0].fee, 10);
    assert_eq!(taken[1].fee, 5);
    assert_eq!(taken[2].fee, 1);
}

#[test]
fn take_selects_highest_fee_when_pool_exceeds_max() {
    let mut pool = Mempool::new(100);

    for (i, &fee) in [1u64, 8, 3, 10, 2].iter().enumerate() {
        let wallet = Wallet::new();
        let amount = i as u64 + 1;
        let mut tx = Transaction::new(wallet.address(), "r", amount, fee);
        wallet.sign_transaction(&mut tx).unwrap();
        pool.add(tx, ENOUGH).unwrap();
    }

    let taken      = pool.take(3);
    let taken_fees: Vec<u64> = taken.iter().map(|t| t.fee).collect();
    assert_eq!(taken_fees, vec![10, 8, 3]);
}
