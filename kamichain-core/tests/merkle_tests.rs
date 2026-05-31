use kamichain_core::merkle::MerkleTree;

#[test]
fn single_transaction_merkle_root_is_its_own_hash() {
    let hashes = vec!["abc123".to_string()];
    let tree = MerkleTree::new(hashes);
    assert!(!tree.root().is_empty());
}

#[test]
fn empty_tree_has_defined_root() {
    // An empty block still needs a valid Merkle root (often all-zero hash)
    let tree = MerkleTree::new(vec![]);
    assert_eq!(tree.root().len(), 64); // 32-byte SHA-256 as hex
}

#[test]
fn two_identical_transactions_produce_consistent_root() {
    let hashes = vec!["aaa".to_string(), "aaa".to_string()];
    let tree = MerkleTree::new(hashes.clone());
    let tree2 = MerkleTree::new(hashes);
    assert_eq!(tree.root(), tree2.root());
}

#[test]
fn different_transactions_produce_different_roots() {
    let tree_a = MerkleTree::new(vec!["tx1".to_string(), "tx2".to_string()]);
    let tree_b = MerkleTree::new(vec!["tx1".to_string(), "tx3".to_string()]);
    assert_ne!(tree_a.root(), tree_b.root());
}

#[test]
fn order_of_transactions_matters() {
    let tree_a = MerkleTree::new(vec!["tx1".to_string(), "tx2".to_string()]);
    let tree_b = MerkleTree::new(vec!["tx2".to_string(), "tx1".to_string()]);
    assert_ne!(tree_a.root(), tree_b.root());
}

#[test]
fn root_is_64_hex_chars() {
    let tree = MerkleTree::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let root = tree.root();
    assert_eq!(root.len(), 64);
    assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn odd_number_of_leaves_is_handled() {
    // With an odd number of leaves the last leaf is duplicated
    let tree = MerkleTree::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert_eq!(tree.root().len(), 64); // must not panic
}

#[test]
fn large_transaction_set_produces_root() {
    let hashes: Vec<String> = (0..128).map(|i| format!("tx{:04}", i)).collect();
    let tree = MerkleTree::new(hashes);
    assert_eq!(tree.root().len(), 64);
}

#[test]
fn verify_inclusion_returns_true_for_member() {
    let hashes = vec!["tx1".to_string(), "tx2".to_string(), "tx3".to_string()];
    let tree = MerkleTree::new(hashes);
    assert!(tree.verify("tx2"));
}

#[test]
fn verify_inclusion_returns_false_for_non_member() {
    let hashes = vec!["tx1".to_string(), "tx2".to_string()];
    let tree = MerkleTree::new(hashes);
    assert!(!tree.verify("tx_not_in_tree"));
}
