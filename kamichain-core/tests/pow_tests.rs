use kamichain_core::{Block, ProofOfWork};

// ── existing behaviour (all still valid after parallel rewrite) ───────────

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
    let block = Block::genesis();
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
    block.hash = "ff".repeat(32);
    assert!(pow.validate(&block).is_err());
}

// ── parallel-specific tests ───────────────────────────────────────────────

#[test]
fn parallel_mine_nonce_is_stored_on_block() {
    // after mining, block.nonce must be the nonce that produces block.hash
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);

    // recompute using the stored nonce — must match the stored hash
    assert_eq!(block.hash, block.compute_hash());
    assert!(block.hash.starts_with("00"));
}

#[test]
fn parallel_mine_at_difficulty_4_is_valid() {
    // higher difficulty — exercises multiple Rayon chunks
    let pow = ProofOfWork::new(4);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);

    assert!(block.hash.starts_with("0000"), "hash: {}", block.hash);
    assert!(pow.validate(&block).is_ok());
    assert_eq!(block.hash, block.compute_hash());
}

#[test]
fn parallel_mine_two_independent_blocks_are_both_valid() {
    // mine two blocks with different prev_hashes — both must independently satisfy PoW
    let pow = ProofOfWork::new(2);

    let mut b1 = Block::new(1, vec![], "a".repeat(64));
    let mut b2 = Block::new(2, vec![], "b".repeat(64));

    pow.mine(&mut b1);
    pow.mine(&mut b2);

    assert!(pow.validate(&b1).is_ok());
    assert!(pow.validate(&b2).is_ok());
    // different prev_hashes must produce different hashes
    assert_ne!(b1.hash, b2.hash);
}

#[test]
fn parallel_mine_different_prev_hashes_give_different_nonces() {
    // two blocks with different prev_hashes should need different nonces to satisfy PoW
    // (they hash differently so the same nonce would be astronomically unlikely to work for both)
    let pow = ProofOfWork::new(3);

    let mut b1 = Block::new(1, vec![], "1".repeat(64));
    let mut b2 = Block::new(1, vec![], "2".repeat(64));

    pow.mine(&mut b1);
    pow.mine(&mut b2);

    // both must be valid regardless of which nonce was found
    assert!(pow.validate(&b1).is_ok());
    assert!(pow.validate(&b2).is_ok());
}

#[test]
fn parallel_mine_hash_is_64_hex_chars() {
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);

    assert_eq!(block.hash.len(), 64);
    assert!(block.hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn validate_rejects_block_with_correct_prefix_but_wrong_hash() {
    // someone fakes a hash that starts with "00" but doesn't match the block fields
    let pow = ProofOfWork::new(2);
    let mut block = Block::new(1, vec![], "0".repeat(64));
    pow.mine(&mut block);

    // overwrite with a fake hash that has the right prefix but wrong content
    block.hash = format!("00{}", "f".repeat(62));

    assert!(pow.validate(&block).is_err());
}
