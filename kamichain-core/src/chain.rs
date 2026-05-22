use crate::block::Block;
use crate::error::KamiError;
use crate::pow::ProofOfWork;

pub struct Chain {
    pub blocks: Vec<Block>,
    pub difficulty: usize,
}

impl Chain {
    pub fn new(difficulty: usize) -> Self {
        todo!()
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), KamiError> {
        todo!()
    }

    pub fn latest_block(&self) -> &Block {
        todo!()
    }

    pub fn is_valid(&self) -> Result<(), KamiError> {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    /// Replace this chain with `candidate` if it is longer and valid.
    /// Returns true if the chain was replaced.
    pub fn replace(&mut self, candidate: Vec<Block>) -> bool {
        todo!()
    }
}
