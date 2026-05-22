use kamichain_core::{Block, Chain, ProofOfWork, Transaction};

fn mine_and_add(chain: &mut Chain, txs: Vec<Transaction>) {
    let pow = ProofOfWork::new(chain.difficulty);
    let prev_hash = chain.latest_block().hash.clone();
    let index = chain.len() as u64;
    let mut block = Block::new(index, txs, prev_hash);
    pow.mine(&mut block);
    chain.add_block(block).expect("add_block failed");
}

#[test]
fn new_chain_starts_with_genesis() {
    let chain = Chain::new(2);
    assert_eq!(chain.len(), 1);
    assert_eq!(chain.latest_block().index, 0);
}

#[test]
fn new_chain_is_valid() {
    let chain = Chain::new(2);
    assert!(chain.is_valid().is_ok());
}

#[test]
fn adding_a_mined_block_increases_length() {
    let mut chain = Chain::new(2);
    mine_and_add(&mut chain, vec![]);
    assert_eq!(chain.len(), 2);
}

#[test]
fn chain_with_three_blocks_is_valid() {
    let mut chain = Chain::new(2);
    mine_and_add(&mut chain, vec![]);
    mine_and_add(&mut chain, vec![Transaction::coinbase("alice", 50)]);
    assert!(chain.is_valid().is_ok());
}

#[test]
fn adding_unmined_block_returns_error() {
    let mut chain = Chain::new(3);
    let prev_hash = chain.latest_block().hash.clone();
    let block = Block::new(1, vec![], prev_hash); // not mined
    assert!(chain.add_block(block).is_err());
}

#[test]
fn adding_block_with_wrong_prev_hash_returns_error() {
    let mut chain = Chain::new(2);
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "wrong_hash".to_string());
    pow.mine(&mut block);
    assert!(chain.add_block(block).is_err());
}

#[test]
fn tampered_block_data_invalidates_chain() {
    let mut chain = Chain::new(2);
    mine_and_add(&mut chain, vec![Transaction::coinbase("alice", 50)]);

    // Secretly alter a transaction amount after the fact
    chain.blocks[1].transactions[0].amount = 9999;

    assert!(chain.is_valid().is_err());
}

#[test]
fn tampered_hash_link_invalidates_chain() {
    let mut chain = Chain::new(2);
    mine_and_add(&mut chain, vec![]);
    mine_and_add(&mut chain, vec![]);

    // Break the hash link between block 1 and block 2
    chain.blocks[1].hash = "deadbeef".repeat(8);

    assert!(chain.is_valid().is_err());
}

#[test]
fn replace_accepts_longer_valid_chain() {
    let mut chain_a = Chain::new(2);
    mine_and_add(&mut chain_a, vec![]);

    let mut chain_b = Chain::new(2);
    mine_and_add(&mut chain_b, vec![]);
    mine_and_add(&mut chain_b, vec![]);
    mine_and_add(&mut chain_b, vec![]);

    let candidate = chain_b.blocks.clone();
    let replaced = chain_a.replace(candidate);

    assert!(replaced);
    assert_eq!(chain_a.len(), 4);
}

#[test]
fn replace_rejects_shorter_chain() {
    let mut chain_a = Chain::new(2);
    mine_and_add(&mut chain_a, vec![]);
    mine_and_add(&mut chain_a, vec![]);

    let chain_b = Chain::new(2); // only genesis

    let candidate = chain_b.blocks.clone();
    let replaced = chain_a.replace(candidate);

    assert!(!replaced);
    assert_eq!(chain_a.len(), 3);
}

#[test]
fn replace_rejects_invalid_chain() {
    let mut chain_a = Chain::new(2);
    mine_and_add(&mut chain_a, vec![]);

    let mut chain_b = Chain::new(2);
    mine_and_add(&mut chain_b, vec![]);
    mine_and_add(&mut chain_b, vec![]);

    // Tamper with chain_b before offering it
    chain_b.blocks[1].transactions = vec![Transaction::coinbase("hacker", 999999)];

    let candidate = chain_b.blocks.clone();
    let replaced = chain_a.replace(candidate);

    assert!(!replaced);
}

#[test]
fn latest_block_returns_last_added() {
    let mut chain = Chain::new(2);
    mine_and_add(&mut chain, vec![Transaction::coinbase("bob", 25)]);
    assert_eq!(chain.latest_block().index, 1);
    assert_eq!(chain.latest_block().transactions[0].recipient, "bob");
}

#[test]
fn get_block_returns_genesis_at_index_zero() {
    let chain = Chain::new(2);
    let block = chain.get_block(0).expect("genesis should exist");
    assert_eq!(block.index, 0);
}

#[test]
fn get_block_returns_correct_block_by_index() {
    let mut chain = Chain::new(2);
    mine_and_add(&mut chain, vec![Transaction::coinbase("alice", 50)]);
    mine_and_add(&mut chain, vec![Transaction::coinbase("bob", 50)]);

    let b1 = chain.get_block(1).unwrap();
    let b2 = chain.get_block(2).unwrap();
    assert_eq!(b1.transactions[0].recipient, "alice");
    assert_eq!(b2.transactions[0].recipient, "bob");
}

#[test]
fn get_block_returns_none_for_out_of_range_index() {
    let chain = Chain::new(2);
    assert!(chain.get_block(999).is_none());
}
