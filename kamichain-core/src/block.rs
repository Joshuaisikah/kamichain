use serde::{Deserialize, Serialize};
use crate::transaction::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn new(index: u64, transactions: Vec<Transaction>, prev_hash: impl Into<String>) -> Self {
        todo!()
    }

    pub fn genesis() -> Self {
        todo!()
    }

    pub fn compute_hash(&self) -> String {
        todo!()
    }

    pub fn is_hash_valid(&self) -> bool {
        todo!()
    }
}
