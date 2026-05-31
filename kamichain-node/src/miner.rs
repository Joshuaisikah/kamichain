use kamichain_core::{Block, ProofOfWork, Transaction};
use kamichain_core::error::KamiError;
use crate::mempool::Mempool;
use crate::state::SharedState;

pub const BLOCK_REWARD: u64 = 50;
pub const MAX_TXS_PER_BLOCK: usize = 100;

pub struct Miner {
    pub address: String,
    pub pow: ProofOfWork,
}

impl Miner {
    pub fn new(address: &str, difficulty: usize) -> Self {
        Miner {
            address: address.to_string(),
            pow: ProofOfWork::new(difficulty),
        }
    }

    pub fn mine_block(&self, state: &SharedState, mempool: &Mempool) -> Block {
        let (index, prev_hash) = {
            let s = state.read().unwrap();
            (s.chain.len() as u64, s.chain.latest_block().hash.clone())
        };

        let mut txs = mempool.take(MAX_TXS_PER_BLOCK);
        txs.insert(0, Transaction::coinbase(&self.address, BLOCK_REWARD));

        let mut block = Block::new(index, txs, prev_hash);
        self.pow.mine(&mut block);
        block
    }

    pub fn mine_and_commit(
        &self,
        state: &SharedState,
        mempool: &mut Mempool,
    ) -> Result<Block, KamiError> {
        let block = self.mine_block(state, mempool);

        {
            let mut s = state.write().unwrap();
            s.chain.add_block(block.clone())?;
            s.apply_block(&block);
        }

        for tx in &block.transactions {
            mempool.remove(&tx.id);
        }

        Ok(block)
    }
}
