
 use thiserror::Error;
 #[derive(Error, Debug)]
 pub enum KamiError {
  #[error("Invalid proof of work")]
  InvalidPoW,
  #[error("Invalid chain: {0}")]
  InvalidChain(String),
  #[error("Invalid transaction: {0}")]
  InvalidTransaction(String),
  #[error("Block not found at index: {0}")]
  BlockNotFound(u64),
  #[error("Serialization error: {0}")]
  SerializationError(String),
 }