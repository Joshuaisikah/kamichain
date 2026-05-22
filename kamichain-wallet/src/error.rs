use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("signature verification failed")]
    VerificationFailed,

    #[error("transaction has no signature to verify")]
    MissingSignature,

    #[error("hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}
