use super::transaction::{transaction_deserializer, Transaction, TransactionState};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::TcpStream,
    sync::{Arc, Mutex},
};
use tracing::debug;

/// Points tuple: available points, locked points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Points(pub usize, pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Points {
    pub fn coordinate(&mut self, tx: Transaction, servers: HashSet<String>) -> Result<(), String> {
        // PREPARE TRANSACTION
        let res: Vec<Result<(TransactionState, TcpStream), String>> = servers
            .par_iter()
            .map(|server| Transaction::prepare(&tx, server))
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

        let abort = abort > 0 || proceed < servers.len() / 2;
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
            /*
            - add points(*) -> should be saved for later
            - lock points -> should fail & do nothing
            - free points -> should be saved for later
            - commit points -> should be freed later
            */
            Err("Not implemented".to_string())
        } else {
            // add / remove / lock the points
            Ok(())
        }
    }

    pub fn handle_transaction(
        &mut self,
        _tx: Transaction,
        _coordinator: TcpStream,
    ) -> Result<(), String> {
        // Already received a transaction, locked points and answered the prepare
        // Should now wait for the commit or abort

        // timeout = COMMIT_TIMEOUT

        // add / remove / lock the points
        Err("Not implemented".to_string())
    }
}
