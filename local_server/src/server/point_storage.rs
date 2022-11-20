use std::{
    collections::{HashMap, HashSet},
    io::Write,
    net::TcpStream,
    sync::{Arc, Mutex},
};

use super::{
    message::{
        connect_to, spread_connect_to, sync_with, ConnectRequest, ConnectResponse, SyncRequest,
        SyncResponse,
    },
    point_record::{PointRecord, Points},
    transaction::{Transaction, TransactionAction, TransactionState},
};
use tracing::{debug, error};

pub type PointMap = HashMap<u16, PointRecord>;

#[derive(Debug)]
pub struct PointStorage {
    pub points: PointMap,
    pub servers: HashSet<String>,
    pub self_address: String,
}

impl PointStorage {
    /// Creates a new point storage.
    /// The point storage is initialized with the given address as self address.
    /// If a known server is given, the point storage will connect to it and sync the points with it.
    ///
    /// # Arguments
    ///
    /// * `self_address` - The address of the server.
    /// * `known_server` - An optional address of a known server.
    ///
    /// # Returns
    ///
    /// The point storage.
    pub fn new(self_address: String, known_address: Option<String>) -> Arc<Mutex<Self>> {
        let mut servers = HashSet::new();
        let mut points = PointMap::new();

        if let Some(addr) = known_address {
            servers = connect_to(&self_address, &addr).unwrap();
            points = sync_with(&addr).unwrap();
        } else {
            servers.insert(self_address.clone());
        }

        Arc::new(Mutex::new(PointStorage {
            points,
            servers,
            self_address,
        }))
    }

    /// Gets the point record for the given id.
    pub fn get_point_record(&mut self, client_id: u16) -> &mut PointRecord {
        self.points
            .entry(client_id)
            .or_insert_with(PointRecord::new)
    }

    /// Gets the list of servers associated with the point storage.
    /// It excludes its own address.
    pub fn get_other_servers(&self) -> HashSet<String> {
        self.servers
            .iter()
            .filter(|addr| **addr != self.self_address)
            .cloned()
            .collect()
    }

    /// Adds a new server to the point storage.
    /// It will also spread the new server to all other servers if the request is not a copy.
    pub fn add_connection(&mut self, request: ConnectRequest) -> Result<String, String> {
        debug!("Adding connection: {:?}", &request.addr);

        if !request.copy {
            self.spread_connection(request.addr.clone())?;
        }

        self.servers.insert(request.addr);

        let res = ConnectResponse {
            servers: self.servers.clone(),
        };

        if request.copy {
            Ok(String::from(""))
        } else {
            serde_json::to_string(&res).map_err(|e| e.to_string())
        }
    }

    /// Creates and serializes a new sync response with the current points.
    pub fn sync(&self, _req: SyncRequest) -> Result<String, String> {
        let res = SyncResponse {
            points: self.points.clone(),
        };
        serde_json::to_string(&res).map_err(|_| "Failed to serialize points".to_string())
    }

    /// Spreads the given server address to all other servers.
    pub fn spread_connection(&mut self, addr: String) -> Result<(), String> {
        for server in &self.servers {
            if server == &addr || server == &self.self_address {
                continue;
            }
            if spread_connect_to(&addr, server).is_err() {
                error!("Failed to spread connection to {}", server);
            }
        }

        Ok(())
    }

    /// Check if order can be fulfilled for the received transaction
    ///
    /// # Returns
    /// [mutex guard/arc mutex] of the record for the client
    pub fn take_for(&mut self, transaction: &Transaction) -> Result<Arc<Mutex<Points>>, String> {
        let record = self.get_point_record(transaction.client_id);

        // wait-die verification
        if let Some(etx) = record.transaction.clone() {
            if transaction.older_than(&etx) {
                return Err("Transaction is older than the current one".to_string());
            }
        }

        // FIXME: point_storage is locked while waiting for this lock
        let points = record.points.clone();
        let points = points.lock().unwrap();

        match transaction.action {
            TransactionAction::Add => Ok(()),
            TransactionAction::Lock => {
                if points.0 < transaction.points {
                    Err("Not enough points available".to_string())
                } else {
                    Ok(())
                }
            }
            _ => {
                // Free or Consume
                if points.1 < transaction.points {
                    Err("Not enough points locked".to_string())
                } else {
                    Ok(())
                }
            }
        }?;

        Ok(record.points.clone())
    }

    /// Handles a transaction for the given storage.
    pub fn handle_transaction(
        storage: Arc<Mutex<PointStorage>>,
        transaction: Transaction,
        mut coordinator: TcpStream,
    ) -> Result<(), String> {
        let mut points = storage.lock().map_err(|_| "Failed to lock storage")?;
        let record = points.take_for(&transaction);

        let state = if record.is_ok() {
            TransactionState::Proceed as u8
        } else {
            TransactionState::Abort as u8
        };
        coordinator.write_all(&[state]).map_err(|e| e.to_string())?;

        let record = record?;
        let mut record = record.lock().map_err(|_| "Failed to lock record")?;
        drop(points); // q: Are these dropped when returning err ?. a: Yes (copilot says)
        record.handle_transaction(transaction, coordinator)
    }
}

/*#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}*/
