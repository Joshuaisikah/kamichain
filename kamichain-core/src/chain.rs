use crate::block::Block;
use crate::error::KamiError;
use crate::merkle::MerkleTree;
use crate::pow::ProofOfWork;
use  serde::{Deserialize,Serialize};

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Chain{
    pub blocks: Vec<Block>,
    pub difficulty: usize,
}
impl Chain {
    pub fn new ( difficulty: usize ) -> Chain {
        Chain{
            blocks:vec![Block::genesis()],
            difficulty,
        }
    }
    pub fn latest_block(&self) -> &Block {
        self.blocks.last().unwrap()
    }
    pub fn len(&self) ->usize{
        self.blocks.len()
    }

    pub fn get_block(&self, index:usize)->Option<&Block>{
        self.blocks.get(index)
    }
    pub fn add_block (&mut self, block:Block) -> Result<(),KamiError>{
        let pow = ProofOfWork::new(self.difficulty);
         pow.validate(&block)?;
        if block.prev_hash != self.latest_block().hash {
            return Err(KamiError::InvalidChain("prev_hash does not match the latest block hash".into()));
        }
        self.blocks.push(block);
        Ok(())
    }
    pub fn is_valid(&self) -> Result<(),KamiError>{
        let pow = { ProofOfWork::new(self.difficulty) };
        for i in 1..self.blocks.len() {
            let current = &self.blocks[i];
            let prev = &self.blocks[i - 1];
            if current.prev_hash != prev.hash {
                return Err(KamiError::InvalidPoW);
            }
            let tx_ids: Vec<String> = current.transactions.iter().map(|tx| tx.compute_id()).collect();
            let merkle_root = MerkleTree::new(tx_ids).root();
            if merkle_root != current.merkle_root {
                return Err(KamiError::InvalidChain(format!("Block {} has invalid merkle root", i)));
            }
            if !current.is_hash_valid() {
                return Err(KamiError::InvalidChain(format!("Block {} has invalid hash", i)));
            }
            pow.validate(&current)?;
        }
        Ok(())
    }
    pub fn replace(&mut self, candidate:Vec<Block>) -> bool{
        if candidate.len() <=self.blocks.len(){
            return false;
        }
        let temp = Chain {
            blocks:candidate.clone(),
            difficulty: self.difficulty,

        };
        if temp.is_valid().is_err(){
            return false;
        }
        self.blocks = candidate;
        true
    }
}
