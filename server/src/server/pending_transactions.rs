use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use std_semaphore::Semaphore;

use super::transaction::Transaction;

pub struct PendingTransactions {
    transactions: Mutex<VecDeque<Transaction>>,
    semaphore: Semaphore,
    online: Semaphore,
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
            online: Semaphore::new(1),
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
    pub fn pop(&self) -> Result<Transaction, String> {
        self.online.acquire();
        self.online.release();
        self.semaphore.acquire();
        let mut txs = self
            .transactions
            .lock()
            .expect("Could not lock transactions");
        txs.pop_front()
            .ok_or_else(|| "Could not pop transaction".to_string())
    }

    pub fn disconnect(&self) {
        self.online.acquire();
    }

    pub fn connect(&self) {
        self.online.release();
    }
}

#[cfg(test)]
mod tests {
    use points::{Message, Order, OrderAction};

    use super::*;
    #[test]
    fn test_add_transactions() {
        let pending_transactions = PendingTransactions::new();
        let order = Order::new(1, OrderAction::UsePoints(123));
        let message = Message::LockOrder(order);
        let transaction = Transaction::new("127.0.0.1:9001".to_string(), &message).unwrap();

        let _ = pending_transactions.add(transaction.clone());
        assert_eq!(pending_transactions.transactions.lock().unwrap().len(), 1);

        let my_transaction = pending_transactions.pop().unwrap();
        assert_eq!(&transaction.clone().client_id, &my_transaction.client_id);
        assert_eq!(&transaction.points, &my_transaction.points);
    }
}
