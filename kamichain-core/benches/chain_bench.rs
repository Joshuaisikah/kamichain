use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kamichain_core::{Block, Chain, ProofOfWork};
use kamichain_core::merkle::MarkleTree;

fn bench_block_hashing(c: &mut Criterion) {
    let block = Block::genesis();
    c.bench_function("block_compute_hash", |b| {
        b.iter(|| black_box(block.compute_hash()))
    });
}

fn bench_merkle_root_100_txs(c: &mut Criterion) {
    let hashes: Vec<String> = (0..100).map(|i| format!("tx{:04x}", i)).collect();
    c.bench_function("merkle_root_100_txs", |b| {
        b.iter(|| {
            let tree = MarkleTree::new(black_box(hashes.clone()));
            black_box(tree.root())
        })
    });
}

fn bench_pow_difficulty_2(c: &mut Criterion) {
    let pow = ProofOfWork::new(2);
    c.bench_function("pow_mine_difficulty_2", |b| {
        b.iter(|| {
            let mut block = Block::new(1, vec![], "0".repeat(64));
            pow.mine(&mut block);
            black_box(block.hash)
        })
    });
}

fn bench_chain_validation_10_blocks(c: &mut Criterion) {
    let pow = ProofOfWork::new(2);
    let mut chain = Chain::new(2);
    for i in 1..=10 {
        let prev = chain.latest_block().hash.clone();
        let mut b = Block::new(i, vec![], prev);
        pow.mine(&mut b);
        chain.add_block(b).unwrap();
    }
    c.bench_function("chain_is_valid_10_blocks", |b| {
        b.iter(|| black_box(chain.is_valid()))
    });
}

criterion_group!(
    benches,
    bench_block_hashing,
    bench_merkle_root_100_txs,
    bench_pow_difficulty_2,
    bench_chain_validation_10_blocks,
);
criterion_main!(benches);
