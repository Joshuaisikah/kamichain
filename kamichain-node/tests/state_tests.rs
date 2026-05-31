use kamichain_core::{Block, ProofOfWork, Transaction};
use kamichain_node::state::NodeState;

fn mined_block(state: &NodeState, txs: Vec<Transaction>) -> Block {
    let pow = ProofOfWork::new(state.chain.difficulty);
    let prev_hash = state.chain.latest_block().hash.clone();
    let index = state.chain.len() as u64;
    let mut block = Block::new(index, txs, prev_hash);
    pow.mine(&mut block);
    block
}

#[test]
fn new_state_has_zero_balances() {
    let state = NodeState::new(2);
    assert_eq!(state.balance_of("anyone"), 0);
}

#[test]
fn apply_coinbase_block_credits_miner() {
    let mut state = NodeState::new(2);
    let block = mined_block(&state, vec![Transaction::coinbase("miner", 50)]);
    state.apply_block(&block);
    assert_eq!(state.balance_of("miner"), 50);
}

#[test]
fn apply_transfer_debits_sender_and_credits_recipient() {
    let mut state = NodeState::new(2);

    // Give alice coins via coinbase
    let b1 = mined_block(&state, vec![Transaction::coinbase("alice", 100)]);
    state.chain.add_block(b1.clone()).unwrap();
    state.apply_block(&b1);

    // Transfer 30 from alice to bob (state tests bypass mempool — apply directly)
    let tx = Transaction::new("alice", "bob", 30, 0);
    let b2 = mined_block(&state, vec![Transaction::coinbase("miner", 50), tx]);
    state.apply_block(&b2);

    assert_eq!(state.balance_of("alice"), 70);
    assert_eq!(state.balance_of("bob"), 30);
}

#[test]
fn multiple_blocks_accumulate_correctly() {
    let mut state = NodeState::new(2);

    for _ in 0..3 {
        let block = mined_block(&state, vec![Transaction::coinbase("miner", 50)]);
        state.chain.add_block(block.clone()).unwrap();
        state.apply_block(&block);
    }

    assert_eq!(state.balance_of("miner"), 150);
}

#[test]
fn balance_of_unfunded_address_is_zero() {
    let state = NodeState::new(2);
    assert_eq!(state.balance_of("never_received_anything"), 0);
}

#[test]
fn apply_block_with_multiple_recipients() {
    let mut state = NodeState::new(2);
    let txs = vec![
        Transaction::coinbase("alice", 50),
        Transaction::coinbase("bob", 30),
    ];
    let block = mined_block(&state, txs);
    state.apply_block(&block);
    assert_eq!(state.balance_of("alice"), 50);
    assert_eq!(state.balance_of("bob"), 30);
}
