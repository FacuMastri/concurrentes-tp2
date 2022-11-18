#![allow(dead_code)] // FIXME: remove this
use std::time::Duration;

const PREPARE_TIMEOUT: Duration = Duration::from_millis(1000);
const COMMIT_TIMEOUT: Duration = Duration::from_millis(3000);

pub enum TransactionState {
    Abort,
    Proceed,
    Timeout,
}

pub enum TransactionAction {
    Add,
    Lock,
    Free,
    Consume,
}

pub struct Transaction {
    pub coordinator: String,
    pub timestamp: u64,
    pub client_id: u16,
    pub action: TransactionAction,
    pub points: usize,
}

impl Transaction {
    pub fn older_than(&self, other: &Transaction) -> bool {
        if self.timestamp == other.timestamp {
            self.coordinator < other.coordinator
        } else {
            self.timestamp < other.timestamp
        }
    }
}
