use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::transaction::{transaction_deserializer, Transaction};

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
    pub fn coordinate(&mut self, _transaction: Transaction) -> Result<(), String> {
        /*
        PREPARE TRANSACTION:
        - using send_to( all_servers, transaction )

        RECEIVE RESPONSES
        - any abort aborts the transactions
        - too many timeouts aborts the transaction
        - enough proceeds -> commit transaction

        FINALIZE TRANSACTION
        - send ABORT or COMMIT to all servers

        IF ABORT
        - add points(*) -> should be saved for later
        - lock points -> should fail & do nothing
        - free points -> should be saved for later
        - commit points -> should be freed later

        */

        Err("Not implemented".to_string())
    }
}
