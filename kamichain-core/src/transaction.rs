use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TxType {
    Coinbase,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub tx_type: TxType,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub signature: Option<String>,
}

impl Transaction {
    pub fn new(sender: impl Into<String>, recipient: impl Into<String>, amount: u64) -> Self {
        todo!()
    }

    pub fn coinbase(recipient: impl Into<String>, reward: u64) -> Self {
        todo!()
    }

    pub fn compute_id(&self) -> String {
        todo!()
    }

    pub fn is_coinbase(&self) -> bool {
        todo!()
    }
}
