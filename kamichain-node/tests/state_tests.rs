use kamichain_core::{Block, Transaction};
use kamichain_node::state::NodeState;

#[test]
fn new_state_has_zero_balances() {
    let state = NodeState::new(2);
    assert_eq!(state.balance_of("anyone"), 0);
}

#[test]
fn apply_coinbase_block_credits_miner() {
    let mut state = NodeState::new(2);
    let coinbase = Transaction::coinbase("miner", 50);
    // build a minimal mined block carrying that coinbase
    // (you will need Block + PoW implemented first)
    // state.apply_block(&block);
    // assert_eq!(state.balance_of("miner"), 50);
}

#[test]
fn apply_transfer_debits_sender_and_credits_recipient() {
    let mut state = NodeState::new(2);

    // Mine a coinbase block to give alice some coins first
    // then apply a transfer block alice -> bob
    // assert sender balance decreases, recipient increases
}

#[test]
fn balance_never_goes_negative() {
    // Attempting to apply a block with a transfer that spends more than the
    // sender's balance should either be rejected at mempool admission time
    // or result in the block being invalid — balance_of should stay >= 0.
}

#[test]
fn multiple_blocks_accumulate_correctly() {
    let mut state = NodeState::new(2);
    // Apply three blocks, assert cumulative balances are correct
}

#[test]
fn apply_block_is_idempotent_for_duplicate_blocks() {
    // Applying the same block twice should not double-credit anyone.
    // The chain prevents this, but state should be robust.
}
