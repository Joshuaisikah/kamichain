use kamichain_core::{Block, ProofOfWork};

#[test]
fn mined_block_hash_starts_with_correct_prefix() {
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);
    assert!(block.hash.starts_with("00"), "hash was: {}", block.hash);
}

#[test]
fn mined_block_hash_matches_stored_hash() {
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);
    assert_eq!(block.hash, block.compute_hash());
}

#[test]
fn validate_passes_for_mined_block() {
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);
    assert!(pow.validate(&block).is_ok());
}

#[test]
fn validate_fails_for_unmined_block() {
    let pow = ProofOfWork::new(4);
    let block = Block::genesis(); // genesis has no PoW at difficulty 4
    // May or may not fail depending on genesis hash — test that validate checks properly
    if !block.hash.starts_with("0000") {
        assert!(pow.validate(&block).is_err());
    }
}

#[test]
fn target_prefix_length_equals_difficulty() {
    let pow = ProofOfWork::new(3);
    let prefix = pow.target_prefix();
    assert_eq!(prefix, "000");
    assert_eq!(prefix.len(), 3);
}

#[test]
fn higher_difficulty_produces_longer_prefix() {
    let easy = ProofOfWork::new(1);
    let hard = ProofOfWork::new(4);

    let mut block_easy = Block::new(1, vec![], "0".repeat(64));
    easy.mine(&mut block_easy);
    assert!(block_easy.hash.starts_with("0"));

    let mut block_hard = Block::new(1, vec![], "0".repeat(64));
    hard.mine(&mut block_hard);
    assert!(block_hard.hash.starts_with("0000"));
}

#[test]
fn tampered_block_fails_validation() {
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);

    // Tamper with the stored hash without remining
    block.hash = "ff".repeat(32);
    assert!(pow.validate(&block).is_err());
}
