use crate::block::Block;
use crate::error::KamiError;
pub struct ProofOfWork{
    pub difficulty:usize,
}

impl ProofOfWork{
    pub fn new(difficulty:usize)->ProofOfWork{
        ProofOfWork{
            difficulty,
        }
    }
    pub fn target_prefix(&self) -> String{
        "0".repeat(self.difficulty)
    }
    pub fn mine(&self, block: &mut Block){
        let target = self.target_prefix();
        loop{
            block.hash =block.compute_hash();
            if block.hash.starts_with(&target){
                break;
            }
            block.nonce += 1;
        }
    }
    pub fn validate(&self, block: &Block) -> Result<(), KamiError> {
        if block.hash != block.compute_hash() {
            return Err(KamiError::InvalidPoW);
        }
        if !block.hash.starts_with(&self.target_prefix()) {
            return Err(KamiError::InvalidPoW);
        }
        Ok(())
    }
}