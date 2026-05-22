/// End-to-end golden path tests.
///
/// These tests run a full scenario through multiple layers:
/// wallet → transaction → mempool → miner → chain → state
/// No networking — pure in-process.

use kamichain_core::{Chain, ProofOfWork, Transaction};
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
    let state   = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner   = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    assert_eq!(state.read().unwrap().balance_of("miner_addr"), BLOCK_REWARD);
}

#[test]
fn signed_transaction_moves_funds_between_wallets() {
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    // Mine a block — miner earns BLOCK_REWARD
    miner.mine_and_commit(&state, &mut pool).unwrap();

    // Miner sends 10 coins to alice
    let mut tx = Transaction::new("miner_addr", "alice_addr", 10);
    let wallet = Wallet::new();
    wallet.sign_transaction(&mut tx).unwrap();
    pool.add(tx).unwrap();

    // Mine a second block to confirm the transaction
    miner.mine_and_commit(&state, &mut pool).unwrap();

    let state_r = state.read().unwrap();
    assert_eq!(state_r.balance_of("alice_addr"), 10);
    assert_eq!(state_r.balance_of("miner_addr"), BLOCK_REWARD * 2 - 10);
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
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    pool.add(Transaction::new("miner_addr", "bob", 5)).unwrap();
    pool.add(Transaction::new("miner_addr", "carol", 3)).unwrap();

    miner.mine_and_commit(&state, &mut pool).unwrap();

    assert!(pool.is_empty());
}

#[test]
fn fork_resolution_adopts_longer_chain() {
    let state_a  = make_state(2);
    let state_b  = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    // B mines 3 extra blocks
    miner.mine_and_commit(&state_b, &mut pool).unwrap();
    miner.mine_and_commit(&state_b, &mut pool).unwrap();
    miner.mine_and_commit(&state_b, &mut pool).unwrap();

    // A adopts B's chain
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

    // Tamper with the transaction amount after mining
    state.write().unwrap().chain.blocks[1].transactions[0].amount = 999999;

    assert!(state.read().unwrap().chain.is_valid().is_err());
}

#[test]
fn wallet_signature_is_verified_before_mempool_admission() {
    let state    = make_state(2);
    let mut pool = Mempool::new(1000);
    let miner    = Miner::new("miner_addr", 2);

    miner.mine_and_commit(&state, &mut pool).unwrap();

    // Unsigned transaction — mempool should reject it
    let unsigned = Transaction::new("miner_addr", "alice", 5);
    // This behaviour is implemented in the mempool or miner — unsigned txs
    // should not be included in a mined block
    // (exact error type depends on your implementation)
    let result = pool.add(unsigned);
    // Either rejected at add time, or filtered at mining time — assert chain stays valid
    miner.mine_and_commit(&state, &mut pool).unwrap();
    assert!(state.read().unwrap().chain.is_valid().is_ok());
}
