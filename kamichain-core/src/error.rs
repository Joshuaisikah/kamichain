use thiserror::Error;

#[derive(Debug, Error)]
pub enum KamiError {
    #[error("invalid block hash: expected difficulty {0}, got {1}")]
    InvalidPoW(usize, String),

    #[error("block {0} not found in chain")]
    BlockNotFound(u64),

    #[error("chain validation failed at block {0}: {1}")]
    InvalidChain(u64, String),

    #[error("invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
