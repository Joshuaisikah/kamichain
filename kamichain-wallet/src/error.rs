// kamichain-wallet/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Verification failed")]
    VerificationFailed,

    #[error("Missing signature")]
    MissingSignature,

    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}
