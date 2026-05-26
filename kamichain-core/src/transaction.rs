use serde::{Deserialize, Serialize};
use sha2::{Digest,Sha256};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType{
    Coinbase,
    Transfer,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub tx_type: TxType,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub fee: u64,
    pub pub_key:Option<String>,
    pub signature:Option<String>,
}

impl Transaction {
    pub fn new(sender:&str, recipient:&str, amount:u64) -> Self {
        let id = compute_id(sender,recipient,amount);
        Transaction{
            id,
            tx_type:TxType::Transfer,
            sender:sender.to_string(),
            recipient:recipient.to_string(),
            amount,
            fee: 0,
            pub_key:None,
            signature:None,
        }
    }

    pub fn coinbase(recipient:&str,reward:u64) -> Self {
         let id = compute_id("",recipient,reward);
         Transaction{
             id,
             tx_type:TxType::Coinbase,
             sender:"".to_string(),
             recipient:recipient.to_string(),
             amount:reward,
             fee:0,
             pub_key:None,
             signature:None,

         }
     }
   pub fn compute_id(&self)->String{
       compute_id(&self.sender, &self.recipient, self.amount)
   }
    pub fn is_coinbase(&self) -> bool {
        self.tx_type == TxType::Coinbase
    }
    pub fn is_transfer(&self) -> bool {
        self.tx_type == TxType::Transfer
    }

}
pub fn compute_id(sender:&str,recipient:&str,amount:u64)->String{
    let mut hasher = Sha256::new();
    hasher.update(sender.as_bytes());
    hasher.update(recipient.as_bytes());
    hasher.update(amount.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}