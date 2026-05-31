use kamichain_core::error::KamiError;
use kamichain_core::Transaction;
use kamichain_wallet::Wallet;
use std::collections::HashMap;

pub struct Mempool {
    pending: HashMap<String, Transaction>,
    capacity: usize,
}

impl Mempool {
    pub fn new(capacity: usize) -> Self {
        Mempool {
            pending: HashMap::new(),
            capacity,
        }
    }

    pub fn add(&mut self, tx: Transaction, sender_balance: u64) -> Result<(), KamiError> {
        if tx.is_coinbase() {
            return Err(KamiError::InvalidTransaction(
                "coinbase transactions cannot be added to mempool".into(),
            ));
        }
        if tx.sender == tx.recipient {
            return Err(KamiError::InvalidTransaction(
                "sender and recipient must differ".into(),
            ));
        }
        if tx.amount == 0 {
            return Err(KamiError::InvalidTransaction(
                "amount must be greater than zero".into(),
            ));
        }

        let pub_key_hex = tx
            .pub_key
            .as_ref()
            .ok_or_else(|| KamiError::InvalidTransaction("transaction is not signed".into()))?;
        Wallet::verify_transaction(&tx, pub_key_hex)
            .map_err(|e| KamiError::InvalidTransaction(e.to_string()))?;

        let total = tx
            .amount
            .checked_add(tx.fee)
            .ok_or_else(|| KamiError::InvalidTransaction("amount + fee overflow".into()))?;
        if total > sender_balance {
            return Err(KamiError::InvalidTransaction(format!(
                "insufficient balance: need {}, have {}",
                total, sender_balance
            )));
        }

        if self.pending.contains_key(&tx.id) {
            return Err(KamiError::InvalidTransaction(
                "duplicate transaction".into(),
            ));
        }
        if self.pending.len() >= self.capacity {
            return Err(KamiError::InvalidTransaction("mempool is full".into()));
        }

        self.pending.insert(tx.id.clone(), tx);
        Ok(())
    }

    pub fn take(&self, max: usize) -> Vec<Transaction> {
        let mut txs: Vec<&Transaction> = self.pending.values().collect();
        txs.sort_by_key(|b| std::cmp::Reverse(b.fee));
        txs.into_iter().take(max).cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn contains(&self, tx_id: &str) -> bool {
        self.pending.contains_key(tx_id)
    }

    pub fn remove(&mut self, tx_id: &str) {
        self.pending.remove(tx_id);
    }
}
