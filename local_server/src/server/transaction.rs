#![allow(dead_code)] // FIXME: remove this
use std::time::Duration;

use points::{Message, OrderAction};
use serde::{Deserialize, Deserializer, Serialize};

const PREPARE_TIMEOUT: Duration = Duration::from_millis(1000);
const COMMIT_TIMEOUT: Duration = Duration::from_millis(3000);

#[derive(Debug, Clone)]
pub enum TransactionState {
    Abort,
    Proceed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionAction {
    Add,
    Lock,
    Free,
    Consume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub coordinator: String,
    pub timestamp: u64,
    pub client_id: u16,
    pub action: TransactionAction,
    pub points: usize,
}

impl Transaction {
    pub fn new(coordinator: String, msg: &Message) -> Result<Transaction, String> {
        let err = Err("Invalid message for transaction".to_string());

        let action = match msg {
            Message::LockOrder(order) => match order.action {
                OrderAction::FillPoints(_) => err,
                OrderAction::UsePoints(_) => Ok(TransactionAction::Lock),
            },
            Message::FreeOrder(order) => match order.action {
                OrderAction::FillPoints(_) => err,
                OrderAction::UsePoints(_) => Ok(TransactionAction::Free),
            },
            Message::CommitOrder(order) => match order.action {
                OrderAction::FillPoints(_) => Ok(TransactionAction::Add),
                OrderAction::UsePoints(_) => Ok(TransactionAction::Consume),
            },
        }?;

        let order = msg.order();
        let client_id = order.client_id;
        let points = order.action.points();

        let timestamp = generate_timestamp();

        Ok(Transaction {
            coordinator,
            timestamp,
            client_id,
            action,
            points,
        })
    }

    pub fn older_than(&self, other: &Transaction) -> bool {
        if self.timestamp == other.timestamp {
            self.coordinator < other.coordinator
        } else {
            self.timestamp < other.timestamp
        }
    }
}

fn generate_timestamp() -> u64 {
    1000
}

pub fn transaction_deserializer<'de, D>(_deserializer: D) -> Result<Option<Transaction>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(None)
}
