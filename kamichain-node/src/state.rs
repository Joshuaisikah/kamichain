use std::collections::HashMap;
use std::sync::{Arc,RwLock};
use kamichain_core::{Block, Chain};
use kamichain_core::transaction::TxType;
 pub type SharedState = Arc<RwLock<NodeState>>;
pub struct NodeState{
    pub chain: Chain,
    pub balances :HashMap<String, u64>,
}
impl NodeState{
    pub fn new(difficulty:usize) ->Self{
        NodeState{
            chain: Chain::new(difficulty),
            balances : HashMap::new(),

        }
    }
     pub fn new_shared(difficulty:usize) -> SharedState{
         Arc::new(RwLock::new(NodeState::new(difficulty)))
     }
    pub fn apply_block(&mut self, block: &Block) {
        for tx in &block.transactions {
            match tx.tx_type {
                TxType::Coinbase => {
                    *self.balances.entry(tx.recipient.clone()).or_insert(0) += tx.amount
                },

                TxType::Transfer => {
                    let sender_balance = self.balances.entry(tx.sender.clone()).or_insert(0);
                    *sender_balance = sender_balance.saturating_sub(tx.amount);
                    *self.balances.entry(tx.recipient.clone()).or_insert(0) += tx.amount;
                }
            }
        }
    }
    pub fn balance_of(&self, address: &str) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }
}