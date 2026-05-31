/// End-to-end golden path tests.
///
/// These tests run a full scenario through multiple layers:
/// wallet → transaction → mempool → miner → chain → state
/// No networking — pure in-process.

use kamichain_core::Transaction;
use kamichain_node::mempool::Mempool;
use kamichain_node::miner::{Miner, BLOCK_REWARD};
use kamichain_node::state::NodeState;
use kamichain_wallet::Wallet;
use std::sync::{Arc, RwLock};

fn make_state(difficulty: usize) -> Arc<RwLock<NodeState>> {
    Arc::new(RwLock::new(NodeState::new(difficulty)))
}

#[test]
fn miner_earns_reward_after_mining_first_block() {
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    assert_eq!(state.read().unwrap().balance_of("miner_addr"), BLOCK_REWARD);
}

#[test]
fn signed_transaction_moves_funds_between_wallets() {
    let state        = make_state(2);
    let mut pool     = Mempool::new(1000);
    let miner_wallet = Wallet::new();
    let miner_addr   = miner_wallet.address();
    let miner        = Miner::new(&miner_addr, 2);

    // Mine a block — miner earns BLOCK_REWARD at their real wallet address.
    miner.mine_and_commit(&state, &mut pool).unwrap();

    let alice_addr   = "alice_recipient_address";
    let mut tx       = Transaction::new(&miner_addr, alice_addr, 10, 0);
    miner_wallet.sign_transaction(&mut tx).unwrap();
    let sender_balance = state.read().unwrap().balance_of(&miner_addr);
    pool.add(tx, sender_balance).unwrap();

    // Mine a second block to confirm the transaction.
    miner.mine_and_commit(&state, &mut pool).unwrap();

    let state_r = state.read().unwrap();
    assert_eq!(state_r.balance_of(alice_addr), 10);
    assert_eq!(state_r.balance_of(&miner_addr), BLOCK_REWARD * 2 - 10);
}

#[test]
fn chain_grows_correctly_over_multiple_mine_cycles() {
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    for _ in 0..5 {
        miner.mine_and_commit(&state, &mut pool).unwrap();
    }

    let state_r = state.read().unwrap();
    assert_eq!(state_r.chain.len(), 6); // genesis + 5 mined
    assert!(state_r.chain.is_valid().is_ok());
}

#[test]
fn mempool_is_empty_after_all_txs_confirmed() {
    let state        = make_state(2);
    let mut pool     = Mempool::new(1000);
    let miner_wallet = Wallet::new();
    let miner_addr   = miner_wallet.address();
    let miner        = Miner::new(&miner_addr, 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    let mut tx1 = Transaction::new(&miner_addr, "bob",   5, 0);
    let mut tx2 = Transaction::new(&miner_addr, "carol", 3, 0);
    miner_wallet.sign_transaction(&mut tx1).unwrap();
    miner_wallet.sign_transaction(&mut tx2).unwrap();

    let sender_balance = state.read().unwrap().balance_of(&miner_addr);
    pool.add(tx1, sender_balance).unwrap();
    pool.add(tx2, sender_balance).unwrap();

    miner.mine_and_commit(&state, &mut pool).unwrap();

    assert!(pool.is_empty());
}

#[test]
fn fork_resolution_adopts_longer_chain() {
    let state_a  = make_state(2);
    let state_b  = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state_b, &mut pool).unwrap();
    miner.mine_and_commit(&state_b, &mut pool).unwrap();
    miner.mine_and_commit(&state_b, &mut pool).unwrap();

    let candidate = state_b.read().unwrap().chain.blocks.clone();
    let replaced  = state_a.write().unwrap().chain.replace(candidate);

    assert!(replaced);
    assert_eq!(state_a.read().unwrap().chain.len(), 4);
}

#[test]
fn chain_rejects_tampered_block() {
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    state.write().unwrap().chain.blocks[1].transactions[0].amount = 999999;

    assert!(state.read().unwrap().chain.is_valid().is_err());
}

#[test]
fn wallet_signature_is_verified_before_mempool_admission() {
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    let unsigned = Transaction::new("miner_addr", "alice", 5, 0);
    assert!(
        pool.add(unsigned, 1000).is_err(),
        "unsigned tx must be rejected at mempool admission"
    );

    miner.mine_and_commit(&state, &mut pool).unwrap();
    assert!(state.read().unwrap().chain.is_valid().is_ok());
}
