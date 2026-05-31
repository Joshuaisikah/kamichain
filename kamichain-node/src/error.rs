use kamichain_core::error::KamiError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("config error: {0}")]
    Config(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("p2p error: {0}")]
    P2P(String),

    #[error("rpc error: {0}")]
    Rpc(String),

    #[error("chain error: {0}")]
    Chain(#[from] KamiError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
