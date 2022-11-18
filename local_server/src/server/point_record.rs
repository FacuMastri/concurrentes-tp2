use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::transaction::{transaction_deserializer, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointRecord {
    // available points / locked points
    pub points: Arc<Mutex<(usize, usize)>>,
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "transaction_deserializer")]
    pub transaction: Option<Transaction>,
}

impl PointRecord {
    pub fn new() -> Self {
        PointRecord {
            points: Arc::new(Mutex::new((0, 0))),
            transaction: None,
        }
    }
}
