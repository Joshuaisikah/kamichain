use kamichain_core::{Block, Transaction};

#[test]
fn genesis_block_has_index_zero() {
    let genesis = Block::genesis();
    assert_eq!(genesis.index, 0);
}

#[test]
fn genesis_prev_hash_is_zeros() {
    let genesis = Block::genesis();
    assert_eq!(genesis.prev_hash, "0".repeat(64));
}

#[test]
fn genesis_hash_is_not_empty() {
    let genesis = Block::genesis();
    assert!(!genesis.hash.is_empty());
}

#[test]
fn genesis_hash_matches_recompute() {
    let genesis = Block::genesis();
    assert_eq!(genesis.hash, genesis.compute_hash());
}

#[test]
fn new_block_links_to_parent_hash() {
    let genesis = Block::genesis();
    let block = Block::new(1, vec![], genesis.hash.clone());
    assert_eq!(block.prev_hash, genesis.hash);
}

#[test]
fn new_block_index_increments() {
    let genesis = Block::genesis();
    let block = Block::new(1, vec![], genesis.hash.clone());
    assert_eq!(block.index, 1);
}

#[test]
fn block_hash_changes_when_nonce_changes() {
    let mut block = Block::new(1, vec![], "0".repeat(64));
    let hash_before = block.compute_hash();
    block.nonce += 1;
    let hash_after = block.compute_hash();
    assert_ne!(hash_before, hash_after);
}

#[test]
fn block_hash_changes_when_transaction_added() {
    let mut block = Block::new(1, vec![], "0".repeat(64));
    let hash_before = block.compute_hash();
    block.transactions.push(Transaction::coinbase("alice", 50));
    let hash_after = block.compute_hash();
    assert_ne!(hash_before, hash_after);
}

#[test]
fn block_stores_transactions() {
    let tx = Transaction::coinbase("miner_address", 50);
    let block = Block::new(1, vec![tx.clone()], "0".repeat(64));
    assert_eq!(block.transactions.len(), 1);
    assert_eq!(block.transactions[0].recipient, "miner_address");
}

#[test]
fn block_hash_is_64_hex_chars() {
    let block = Block::genesis();
    assert_eq!(block.hash.len(), 64);
    assert!(block.hash.chars().all(|c| c.is_ascii_hexdigit()));
}
