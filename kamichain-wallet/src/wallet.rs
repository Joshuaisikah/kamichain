use kamichain_core::Transaction;
use crate::error::WalletError;

pub struct Wallet {
    signing_key: ed25519_dalek::SigningKey,
}

impl Wallet {
    pub fn new() -> Self {
        todo!()
    }

    pub fn address(&self) -> String {
        todo!()
    }

    pub fn public_key_hex(&self) -> String {
        todo!()
    }

    pub fn sign_transaction(&self, tx: &mut Transaction) -> Result<(), WalletError> {
        todo!()
    }

    pub fn verify_transaction(tx: &Transaction, public_key_hex: &str) -> Result<bool, WalletError> {
        todo!()
    }
}
