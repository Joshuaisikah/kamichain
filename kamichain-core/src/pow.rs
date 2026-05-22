use crate::block::Block;
use crate::error::KamiError;

pub struct ProofOfWork {
    pub difficulty: usize,
}

impl ProofOfWork {
    pub fn new(difficulty: usize) -> Self {
        todo!()
    }

    pub fn mine(&self, block: &mut Block) {
        todo!()
    }

    pub fn validate(&self, block: &Block) -> Result<(), KamiError> {
        todo!()
    }

    pub fn target_prefix(&self) -> String {
        todo!()
    }
}
