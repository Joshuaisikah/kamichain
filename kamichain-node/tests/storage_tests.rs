use kamichain_core::{Block, Chain, ProofOfWork};
use kamichain_node::storage::Storage;
use std::path::PathBuf;

fn tmp_path() -> PathBuf {
    std::env::temp_dir().join(format!("kamichain_test_{}.json", rand_suffix()))
}

fn rand_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as u64
}

fn mine_block(chain: &Chain) -> Block {
    let pow = ProofOfWork::new(chain.difficulty);
    let prev = chain.latest_block().hash.clone();
    let idx = chain.len() as u64;
    let mut b = Block::new(idx, vec![], prev);
    pow.mine(&mut b);
    b
}

#[test]
fn save_and_load_roundtrips_genesis_chain() {
    let path = tmp_path();
    let storage = Storage::new(&path);
    let chain = Chain::new(2);

    storage.save_chain(&chain).unwrap();
    let loaded = storage.load_chain().unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.latest_block().hash, chain.latest_block().hash);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_and_load_roundtrips_multi_block_chain() {
    let path = tmp_path();
    let storage = Storage::new(&path);
    let mut chain = Chain::new(2);

    chain.add_block(mine_block(&chain)).unwrap();
    chain.add_block(mine_block(&chain)).unwrap();

    storage.save_chain(&chain).unwrap();
    let loaded = storage.load_chain().unwrap();

    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded.latest_block().hash, chain.latest_block().hash);
    assert!(loaded.is_valid().is_ok());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_returns_error_when_file_missing() {
    let storage = Storage::new("/tmp/kamichain_does_not_exist_xyz.json");
    assert!(storage.load_chain().is_err());
}

#[test]
fn save_overwrites_previous_file() {
    let path = tmp_path();
    let storage = Storage::new(&path);

    let chain_a = Chain::new(2);
    storage.save_chain(&chain_a).unwrap();

    let mut chain_b = Chain::new(2);
    chain_b.add_block(mine_block(&chain_b)).unwrap();
    storage.save_chain(&chain_b).unwrap();

    let loaded = storage.load_chain().unwrap();
    assert_eq!(loaded.len(), 2);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn loaded_chain_passes_validation() {
    let path = tmp_path();
    let storage = Storage::new(&path);
    let mut chain = Chain::new(2);
    chain.add_block(mine_block(&chain)).unwrap();

    storage.save_chain(&chain).unwrap();
    let loaded = storage.load_chain().unwrap();

    assert!(loaded.is_valid().is_ok());
    let _ = std::fs::remove_file(&path);
}
