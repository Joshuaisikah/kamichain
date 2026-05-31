use crate::error::WalletError;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use kamichain_core::Transaction;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::path::Path;
pub struct Wallet {
    signing_key: SigningKey,
}
impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

impl Wallet {
    pub fn new() -> Self {
        Self {
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    pub fn address(&self) -> String {
        let pub_key_bytes = self.signing_key.verifying_key().to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(pub_key_bytes);
        format!("{:x}", hasher.finalize())
    }
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }
    pub fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), WalletError> {
        let message = format!("{}{}{}{}", tx.sender, tx.recipient, tx.amount, tx.id);
        let signature = self.signing_key.sign(message.as_bytes());
        tx.signature = Some(hex::encode(signature.to_bytes()));
        tx.pub_key = Some(self.public_key_hex());
        Ok(())
    }
    pub fn verify_transaction(tx: &Transaction, pub_key_hex: &str) -> Result<bool, WalletError> {
        let sig_hex = tx.signature.as_ref().ok_or(WalletError::MissingSignature)?;
        let pub_key_bytes = hex::decode(pub_key_hex)?;
        let pub_key_array: [u8; 32] = pub_key_bytes
            .try_into()
            .map_err(|_| WalletError::InvalidPublicKey("Key must be 32 bytes".into()))?;
        let mut hasher = Sha256::new();
        hasher.update(pub_key_array);
        let derived_address = format!("{:x}", hasher.finalize());
        if derived_address != tx.sender {
            return Err(WalletError::InvalidPublicKey(
                "Public key does not match sender address".into(),
            ));
        }

        let verifying_key = VerifyingKey::from_bytes(&pub_key_array)
            .map_err(|_| WalletError::InvalidPublicKey("Invalid public key bytes".into()))?;
        let message = format!("{}{}{}{}", tx.sender, tx.recipient, tx.amount, tx.id);
        let sig_bytes = hex::decode(sig_hex)?;
        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| WalletError::VerificationFailed)?;
        let signature = Signature::from_bytes(&sig_array);
        verifying_key
            .verify(message.as_bytes(), &signature)
            .map_err(|_| WalletError::VerificationFailed)?;
        Ok(true)
    }
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), WalletError> {
        let hex = hex::encode(self.signing_key.to_bytes());
        std::fs::write(path, hex).map_err(|e| WalletError::InvalidPublicKey(e.to_string()))
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Wallet, WalletError> {
        let hex = std::fs::read_to_string(path)
            .map_err(|e| WalletError::InvalidPublicKey(e.to_string()))?;
        let bytes = hex::decode(hex.trim())?;
        let array: [u8; 32] = bytes
            .try_into()
            .map_err(|_| WalletError::InvalidPublicKey("Key must be 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes(&array);
        Ok(Wallet { signing_key })
    }
}
