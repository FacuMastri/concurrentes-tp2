use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use super::transaction::Transaction;

#[derive(Debug)]
pub struct PendingTransactions {
    transactions: Mutex<VecDeque<Transaction>>,
}

impl PendingTransactions {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            transactions: Mutex::new(VecDeque::new()),
        })
    }

    pub fn add(&self, transaction: Transaction) -> Result<(), String> {
        let mut txs = self
            .transactions
            .lock()
            .map_err(|_| "Could not lock transactions")?;
        txs.push_back(transaction);
        Ok(())
    }
}
