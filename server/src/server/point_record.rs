use super::{
    pending_transactions::PendingTransactions,
    transaction::{Transaction, TransactionAction, TransactionState, TxOk, COMMIT_TIMEOUT},
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
use tracing::{debug, info, warn};

/// Points tuple: available points, locked points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Points(pub usize, pub usize);

#[derive(Clone, Serialize, Deserialize)]
pub struct PointRecord {
    pub points: Arc<Mutex<Points>>,
    pub transaction: Option<Transaction>,
}

impl PointRecord {
    pub fn new() -> Self {
        PointRecord {
            points: Arc::new(Mutex::new(Points(0, 0))),
            transaction: None,
        }
    }

    pub fn wait_die(&self, transaction: &Transaction) -> Result<(), String> {
        if let Some(etx) = self.transaction.clone() {
            if transaction.older_than(&etx) {
                return Err("Transaction is older than the current one".to_string());
            }
        }
        Ok(())
    }
}

impl Points {
    /// Prepares the transaction
    /// Returns (abort, streams)
    fn prepare(
        &mut self,
        transaction: Transaction,
        servers: HashSet<String>,
        online: bool,
    ) -> Result<(TransactionState, Vec<Result<TcpStream, String>>), String> {
        if !online {
            return Ok((TransactionState::Disconnected, vec![]));
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
            .map(|res| match res {
                Ok((state, stream)) => {
                    match state {
                        TransactionState::Proceed => {
                            debug!(
                                "Received APPROVE message for transaction with timestamp {}.",
                                transaction.timestamp
                            );
                            proceed += 1
                        }
                        TransactionState::Abort => {
                            debug!(
                                "Received ABORT message for transaction with timestamp {}.",
                                transaction.timestamp
                            );
                            abort += 1
                        }
                        _ => {}
                    }
                    Ok(stream)
                }
                Err(e) => Err(e),
            })
            .collect();

        // Evaluate if the transaction should be aborted or committed
        if abort == 0 && proceed == 0 {
            return Ok((TransactionState::Disconnected, streams));
        }

        let abort = abort > 0 || proceed < servers.len() / 2;
        let state = if abort {
            debug!(
                "Coordinator decided to ABORT transaction with timestamp {}.",
                transaction.timestamp
            );
            TransactionState::Abort
        } else {
            debug!(
                "Coordinator decided to COMMIT transaction with timestamp {}.",
                transaction.timestamp
            );
            TransactionState::Proceed
        };
        Ok((state, streams))
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
        pending: Arc<PendingTransactions>,
    ) -> Result<TxOk, String> {
        self.can_perform(&transaction)?;

        // Commit the transaction directly if this is the only server
        if servers.is_empty() {
            self.apply(transaction);
            return Ok(TxOk::Finalized);
        }

        // PREPARE TRANSACTION
        let (state, streams) = self.prepare(transaction.clone(), servers, online)?;

        // FINALIZE TRANSACTION
        for stream in streams {
            match stream {
                Ok(mut stream) => {
                    let _ = Transaction::finalize(&mut stream, state.clone());
                }
                Err(err) => {
                    warn!(err)
                }
            }
        }

        match state {
            TransactionState::Proceed => {
                pending.connect();
                self.apply(transaction);
                Ok(TxOk::Finalized)
            }
            TransactionState::Abort => {
                pending.connect();
                match transaction.action {
                    TransactionAction::Lock => Err("Transaction Aborted".to_string()),
                    _ => {
                        pending.add(transaction)?;
                        Ok(TxOk::Pending)
                    }
                }
            }
            TransactionState::Disconnected => {
                pending.disconnect();
                match transaction.action {
                    TransactionAction::Lock => Err("Transaction Aborted".to_string()),
                    _ => {
                        pending.add(transaction)?;
                        Ok(TxOk::Pending)
                    }
                }
            }
            _ => Err("Invalid TransactionState".to_string()),
        }
    }

    pub fn can_perform(&self, transaction: &Transaction) -> Result<(), String> {
        match transaction.action {
            TransactionAction::Add => Ok(()),
            TransactionAction::Lock => {
                if self.0 < transaction.points {
                    Err("Not enough points available".to_string())
                } else {
                    Ok(())
                }
            }
            _ => {
                // Free or Consume
                if self.1 < transaction.points {
                    Err("Not enough points locked".to_string())
                } else {
                    Ok(())
                }
            }
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
            debug!(
                "Received COMMIT message from coordinator for transaction with timestamp {}.",
                transaction.timestamp
            );
            self.apply(transaction);
            Ok(())
        } else {
            debug!(
                "Received ABORT message from coordinator for transaction with timestamp {}.",
                transaction.timestamp
            );
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
        info!("Applied {:?}.", transaction);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SafePointRecord(pub Arc<Mutex<PointRecord>>);

impl fmt::Debug for SafePointRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let record = self.0.lock().map_err(|_| fmt::Error)?;
        let points = record.points.clone();
        let points = points.lock().map_err(|_| fmt::Error)?;
        write!(f, "{:?} Available [{:?} Locked]", points.0, points.1)
    }
}

impl SafePointRecord {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(PointRecord::new())))
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
        assert_eq!(0, points.1);
    }

    #[test]
    fn test_lock_points() {
        let mut points = Points(100, 0);
        let order = Order::new(1, OrderAction::UsePoints(100));
        let message = Message::LockOrder(order);
        let transaction = Transaction::new("127.0.0.1:9001".to_string(), &message).unwrap();
        points.apply(transaction);
        assert_eq!(0, points.0);
        assert_eq!(100, points.1);
    }

    #[test]
    fn test_free_points() {
        let mut points = Points(0, 100);
        let order = Order::new(1, OrderAction::UsePoints(100));
        let message = Message::FreeOrder(order);
        let transaction = Transaction::new("127.0.0.1:9001".to_string(), &message).unwrap();
        points.apply(transaction);
        assert_eq!(100, points.0);
        assert_eq!(0, points.1);
    }

    #[test]
    fn test_consume_points() {
        let mut points = Points(0, 100);
        let order = Order::new(1, OrderAction::UsePoints(100));
        let message = Message::CommitOrder(order);
        let transaction = Transaction::new("127.0.0.1:9001".to_string(), &message).unwrap();
        points.apply(transaction);
        assert_eq!(0, points.0);
        assert_eq!(0, points.1);
    }
}
