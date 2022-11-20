use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use std_semaphore::Semaphore;

use super::transaction::Transaction;

pub struct PendingTransactions {
    transactions: Mutex<VecDeque<Transaction>>,
    semaphore: Semaphore,
}

impl std::fmt::Debug for PendingTransactions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.transactions.lock().unwrap().iter())
            .finish()
    }
}

impl PendingTransactions {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            transactions: Mutex::new(VecDeque::new()),
            semaphore: Semaphore::new(0),
        })
    }

    /// Adds an transaction to the queue.
    pub fn add(&self, transaction: Transaction) -> Result<(), String> {
        let mut txs = self
            .transactions
            .lock()
            .map_err(|_| "Could not lock transactions")?;
        txs.push_back(transaction);
        self.semaphore.release();
        Ok(())
    }

    /// Returns the next transaction in the queue.
    /// If there are no transactions, the thread will be blocked until there is one.
    pub fn _pop(&self) -> Result<Transaction, String> {
        self.semaphore.acquire();
        let mut txs = self
            .transactions
            .lock()
            .expect("Could not lock transactions");
        txs.pop_front()
            .ok_or_else(|| "Could not pop transaction".to_string())
    }
}
