use serde::{Deserialize,Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::transaction::Transaction;
use crate::merkle::MerkleTree;
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct Block{
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub merkle_root: String,
    pub prev_hash:String,
    pub hash: String,
    pub nonce: u64,

}
impl Block{
    pub fn genesis()->Self{
        let mut block = Block{
            index: 0,
            timestamp: 0,
            transactions:vec![],
            merkle_root:MerkleTree::new(vec![]).root(),
            prev_hash: "0".repeat(64),
            hash: String::new(),
            nonce: 0,
        };
        block.hash = block.compute_hash();
        block
    }
    pub fn new(index: u64, transactions: Vec<Transaction>,prev_hash:String) ->Self{
        let tx_ids = transactions.iter().map(|tx|tx.id.clone()).collect();
        let merkle_root =MerkleTree::new(tx_ids).root();
        let mut block = Block{
            index,
            timestamp:now(),
            transactions,
            merkle_root,
            prev_hash,
            hash: String::new(),
            nonce: 0,

        };
        block.hash = block.compute_hash();
        block
    }

    pub  fn compute_hash(&self)->String{
        let input = format!("{}{}{}{}{}",self.index,self.timestamp,self.merkle_root,self.prev_hash,self.nonce);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    pub fn is_hash_valid(&self) -> bool {
        self.hash == self.compute_hash()
    }
}
fn now()->u64{
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}