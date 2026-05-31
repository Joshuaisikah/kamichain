use kamichain_core::{ProofOfWork, Transaction};
use kamichain_node::mempool::Mempool;
use kamichain_node::miner::{Miner, BLOCK_REWARD};
use kamichain_node::state::NodeState;
use kamichain_wallet::Wallet;
use std::sync::{Arc, RwLock};

const ENOUGH: u64 = 10_000;

fn make_state(difficulty: usize) -> Arc<RwLock<NodeState>> {
    Arc::new(RwLock::new(NodeState::new(difficulty)))
}

#[test]
fn mined_block_satisfies_pow() {
    let state = make_state(2);
    let mempool = Mempool::new(100);
    let miner = Miner::new("miner_address", 2);

    let block = miner.mine_block(&state, &mempool);
    let pow = ProofOfWork::new(2);
    assert!(pow.validate(&block).is_ok(), "hash: {}", block.hash);
}

#[test]
fn mined_block_includes_coinbase_transaction() {
    let state = make_state(2);
    let mempool = Mempool::new(100);
    let miner = Miner::new("miner_address", 2);

    let block = miner.mine_block(&state, &mempool);
    assert!(!block.transactions.is_empty());
    assert!(block.transactions[0].is_coinbase());
}

#[test]
fn coinbase_reward_goes_to_miner_address() {
    let state = make_state(2);
    let mempool = Mempool::new(100);
    let miner = Miner::new("miner_address", 2);

    let block = miner.mine_block(&state, &mempool);
    assert_eq!(block.transactions[0].recipient, "miner_address");
    assert_eq!(block.transactions[0].amount, BLOCK_REWARD);
}

#[test]
fn mined_block_links_to_latest_chain_block() {
    let state = make_state(2);
    let mempool = Mempool::new(100);
    let miner = Miner::new("miner_address", 2);

    let latest_hash = state.read().unwrap().chain.latest_block().hash.clone();
    let block = miner.mine_block(&state, &mempool);
    assert_eq!(block.prev_hash, latest_hash);
}

#[test]
fn mine_and_commit_adds_block_to_chain() {
    let state = make_state(2);
    let mut mempool = Mempool::new(100);
    let miner = Miner::new("miner_address", 2);

    let len_before = state.read().unwrap().chain.len();
    miner.mine_and_commit(&state, &mut mempool).unwrap();
    let len_after = state.read().unwrap().chain.len();

    assert_eq!(len_after, len_before + 1);
}

#[test]
fn mine_and_commit_includes_mempool_transactions() {
    let state = make_state(2);
    let mut mempool = Mempool::new(100);

    let wallet = Wallet::new();
    let mut tx1 = Transaction::new(wallet.address(), "bob", 5, 0);
    let mut tx2 = Transaction::new(wallet.address(), "carol", 3, 0);
    wallet.sign_transaction(&mut tx1).unwrap();
    wallet.sign_transaction(&mut tx2).unwrap();
    mempool.add(tx1, ENOUGH).unwrap();
    mempool.add(tx2, ENOUGH).unwrap();

    let miner = Miner::new("miner_address", 2);
    let block = miner.mine_and_commit(&state, &mut mempool).unwrap();

    // coinbase + 2 transfers
    assert_eq!(block.transactions.len(), 3);
}

#[test]
fn mine_and_commit_removes_included_txs_from_mempool() {
    let state = make_state(2);
    let mut mempool = Mempool::new(100);

    let wallet = Wallet::new();
    let mut tx = Transaction::new(wallet.address(), "bob", 5, 0);
    wallet.sign_transaction(&mut tx).unwrap();
    mempool.add(tx, ENOUGH).unwrap();

    let miner = Miner::new("miner_address", 2);
    miner.mine_and_commit(&state, &mut mempool).unwrap();

    assert!(mempool.is_empty());
}

#[test]
fn miner_balance_increases_after_mining() {
    let state = make_state(2);
    let mut mempool = Mempool::new(100);
    let miner = Miner::new("miner_address", 2);

    miner.mine_and_commit(&state, &mut mempool).unwrap();

    let balance = state.read().unwrap().balance_of("miner_address");
    assert_eq!(balance, BLOCK_REWARD);
}
