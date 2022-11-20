use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use points::{Message, OrderAction};
use serde::{Deserialize, Deserializer, Serialize};

use super::message::{write_message_to, TRANSACTION};

pub const PREPARE_TIMEOUT: Duration = Duration::from_millis(1000);
pub const COMMIT_TIMEOUT: Duration = Duration::from_millis(3000);

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
    pub timestamp: u128,
    pub client_id: u16,
    pub action: TransactionAction,
    pub points: usize,
}

impl Transaction {
    /// Creates a new transaction with the given coordinator as the origin address and
    /// the given message as the transaction action.
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

    /// Compares the given transaction's timestamp with this transaction's timestamp.
    /// Returns true if the given transaction's timestamp is greater than this transaction's timestamp.
    /// In case of a tie, the transaction with the lower coordinator is considered greater.
    pub fn older_than(&self, other: &Transaction) -> bool {
        if self.timestamp == other.timestamp {
            self.coordinator < other.coordinator
        } else {
            self.timestamp < other.timestamp
        }
    }

    /// Sends a transaction message to the given server address.
    pub fn prepare(
        transaction: &Transaction,
        server: &String,
    ) -> Result<(TransactionState, TcpStream), String> {
        let mut stream = write_message_to(TRANSACTION, transaction, server)?;
        stream
            .set_read_timeout(Some(PREPARE_TIMEOUT))
            .map_err(|e| e.to_string())?;

        let mut buf = [0u8; 1];
        let read = stream.read_exact(&mut buf);
        if read.is_err() {
            return Ok((TransactionState::Timeout, stream));
        }
        if buf[0] == TransactionState::Proceed as u8 {
            Ok((TransactionState::Proceed, stream))
        } else {
            Ok((TransactionState::Abort, stream))
        }
    }

    /// Sends a transaction state message to the given stream.
    pub fn finalize(stream: &mut TcpStream, state: TransactionState) -> Result<(), String> {
        stream.write_all(&[state as u8]).map_err(|e| e.to_string())
    }
}

fn generate_timestamp() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    since_the_epoch.as_millis()
}

pub fn transaction_deserializer<'de, D>(_deserializer: D) -> Result<Option<Transaction>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(None)
}

#[cfg(test)]
mod tests {
    use points::Order;

    use super::*;
    #[test]
    fn test_transaction() {
        let order = Order::new(1, OrderAction::UsePoints(123));
        let message = Message::LockOrder(order);
        let transaction = Transaction::new("127.0.0.1:9001".to_string(), &message).unwrap();

        let other_order = Order::new(1, OrderAction::UsePoints(123));
        let other_message = Message::LockOrder(other_order);
        let other_transaction =
            Transaction::new("127.0.0.1:9002".to_string(), &other_message).unwrap();

        assert_eq!(true, transaction.older_than(&other_transaction));
    }
}
