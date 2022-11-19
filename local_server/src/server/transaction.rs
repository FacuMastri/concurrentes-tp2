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

    pub fn prepare(
        tx: &Transaction,
        server: &String,
    ) -> Result<(TransactionState, TcpStream), String> {
        let mut stream = write_message_to(TRANSACTION, tx, server)?;
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
