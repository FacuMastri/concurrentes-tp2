use super::transaction::{
    transaction_deserializer, Transaction, TransactionAction, TransactionState, COMMIT_TIMEOUT,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    io::Read,
    net::TcpStream,
    sync::{Arc, Mutex},
};
use tracing::debug;

/// Points tuple: available points, locked points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Points(pub usize, pub usize);

#[derive(Clone, Serialize, Deserialize)]
pub struct PointRecord {
    // available points / locked points
    pub points: Arc<Mutex<Points>>,
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "transaction_deserializer")]
    pub transaction: Option<Transaction>,
}

impl PointRecord {
    pub fn new() -> Self {
        PointRecord {
            points: Arc::new(Mutex::new(Points(0, 0))),
            transaction: None,
        }
    }
}

impl fmt::Debug for PointRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let points = self.points.lock().map_err(|_| fmt::Error)?;
        write!(f, "{:?} [{:?}]", points.0, points.1)
    }
}

impl Points {
    fn prepare(
        &mut self,
        transaction: Transaction,
        servers: HashSet<String>,
        online: bool,
    ) -> Result<(bool, Vec<Result<TcpStream, String>>), String> {
        if !online {
            return Ok((true, vec![]));
        }

        // PREPARE TRANSACTION

        let res: Vec<Result<(TransactionState, TcpStream), String>> = servers
            .par_iter()
            .map(|server| Transaction::prepare(&transaction, server))
            .collect();

        // Evaluate the results. If any of the servers failed to prepare, abort the transaction.
        // If less than half timeout, proceed to commit.

        let mut proceed = 0;
        let mut abort = 0;

        let streams: Vec<Result<TcpStream, String>> = res
            .into_iter()
            .map(|res| {
                if let Ok((state, stream)) = res {
                    match state {
                        TransactionState::Proceed => proceed += 1,
                        TransactionState::Abort => abort += 1,
                        _ => {}
                    }
                    Ok(stream)
                } else {
                    abort += 1;
                    Err("Error".to_string())
                }
            })
            .collect();

        // Evaluate if the transaction should be aborted or committed
        let abort = abort > 0 || proceed < servers.len() / 2;
        Ok((abort, streams))
    }

    /// Coordinates a transaction among all other servers
    /// The algorithm works as follows:
    /// 1. The coordinator sends a prepare message to all other servers
    /// 2. Each server responds with a proceed message if it can commit the transaction
    /// 3. If all servers (or if less than half of them timeout) respond with proceed, the coordinator sends a commit message to all server
    /// 3.1 If any server responds with an abort, the coordinator sends an abort message to all servers
    pub fn coordinate(
        &mut self,
        transaction: Transaction,
        servers: HashSet<String>,
        online: bool,
    ) -> Result<(), String> {
        // PREPARE TRANSACTION
        let (abort, streams) = self.prepare(transaction.clone(), servers, online)?;

        let state = if abort {
            TransactionState::Abort
        } else {
            TransactionState::Proceed
        };

        // FINALIZE TRANSACTION
        for stream in streams {
            let mut stream = stream.expect("Should not fail");
            let res = Transaction::finalize(&mut stream, state.clone());
            if res.is_err() {
                debug!("Error finalizing transaction");
            }
        }

        if abort {
            match transaction.action {
                TransactionAction::Lock => Err("Transaction Aborted".to_string()),
                _ => {
                    // TODO: handle abort
                    // Save for later
                    // Ok(())
                    Err("Not implemented".to_string())
                }
            }
        } else {
            self.apply(transaction);
            Ok(())
        }
    }

    /// Handles a transaction waiting for a commit message or an abort message.
    /// If the transaction is aborted, the transaction is discarded.
    /// If the transaction is committed, the transaction is applied to the points.
    pub fn handle_transaction(
        &mut self,
        transaction: Transaction,
        mut coordinator: TcpStream,
    ) -> Result<(), String> {
        // Already received a transaction, locked points and answered the prepare
        // Should now wait for the commit (for a fixed period of time) or abort
        coordinator
            .set_read_timeout(Some(COMMIT_TIMEOUT))
            .expect("Should not fail");

        let mut buf = [TransactionState::Timeout as u8; 1];
        coordinator
            .read_exact(&mut buf)
            .map_err(|e| e.to_string())?;

        if buf[0] == TransactionState::Proceed as u8 {
            self.apply(transaction);
            Ok(())
        } else {
            Err("Aborted Transaction".to_string())
        }
    }

    /// Applies a transaction to the points
    /// If the transaction is a lock, the points are locked (increasing the locked points and decreasing the available points)
    /// If the transaction is free, the points are unlocked (decreasing the locked points and increasing the available points)
    /// If the transaction is an add, the points are added (increasing the available points)
    /// If the transaction is a consume, the points are subtracted (decreasing the locked points)
    pub fn apply(&mut self, transaction: Transaction) {
        match transaction.action {
            TransactionAction::Add => {
                self.0 += transaction.points;
            }
            TransactionAction::Lock => {
                self.0 -= transaction.points;
                self.1 += transaction.points;
            }
            TransactionAction::Free => {
                self.0 += transaction.points;
                self.1 -= transaction.points;
            }
            TransactionAction::Consume => {
                self.1 -= transaction.points;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use points::{Message, Order, OrderAction};

    use super::*;
    #[test]
    fn test_add_points() {
        let mut points = Points(0, 0);
        let order = Order::new(1, OrderAction::FillPoints(100));
        let message = Message::CommitOrder(order);
        let transaction = Transaction::new("127.0.0.1:9001".to_string(), &message).unwrap();
        points.apply(transaction);
        assert_eq!(100, points.0);
    }
}
