use std::{
    collections::{HashMap, HashSet},
    io::Write,
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::{
    message::{
        connect_to, spread_connect_to, sync_with, ConnectRequest, ConnectResponse, SyncRequest,
        SyncResponse, TIMEOUT,
    },
    pending_transactions::PendingTransactions,
    point_record::PointRecord,
    transaction::{Transaction, TransactionState, TxOk},
};
use points::Message;
use tracing::{debug, error, info};

pub type PointMap = HashMap<u16, Arc<Mutex<PointRecord>>>;

#[derive(Debug)]
pub struct PointStorage {
    pub points: PointMap,
    pub servers: HashSet<String>,
    pub self_address: String,
    pub online: bool,
    pub pending: Arc<PendingTransactions>,
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

        let res = Arc::new(Mutex::new(PointStorage {
            points,
            servers,
            self_address,
            online: true,
            pending: PendingTransactions::new(),
        }));

        Self::set_on_connect(res.clone());

        res
    }

    /// Gets the point record for the given id.
    pub fn get_point_record(&mut self, client_id: u16) -> Arc<Mutex<PointRecord>> {
        self.points
            .entry(client_id)
            .or_insert_with(|| Arc::new(Mutex::new(PointRecord::new())))
            .clone()
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
        self.check_online()?;
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
        self.check_online()?;
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

    /*
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
    */
    /// Handles a transaction for the given storage.
    pub fn handle_transaction(
        storage: Arc<Mutex<PointStorage>>,
        transaction: Transaction,
        mut coordinator: TcpStream,
    ) -> Result<(), String> {
        let mut storage = storage.lock().map_err(|_| "Failed to lock storage")?;
        storage.check_online()?;

        let record = storage.get_point_record(transaction.client_id);
        drop(storage);
        let record = record.lock().map_err(|_| "Failed to lock record")?;

        let wait_die = record.wait_die(&transaction);

        let points = record.points.clone();
        let mut points = points.lock().map_err(|_| "Failed to lock points")?;
        drop(record);

        let state = if wait_die.is_ok() && points.can_perform(&transaction).is_ok() {
            debug!("Sending APPROVE for {:?}.", transaction);
            TransactionState::Proceed as u8
        } else {
            debug!("Sending ABORT for {:?}.", transaction);
            TransactionState::Abort as u8
        };
        coordinator.write_all(&[state]).map_err(|e| e.to_string())?;

        points.handle_transaction(transaction, coordinator)
    }

    /// Makes the storage go offline.
    /// It wont send or receive any transactions.
    pub fn disconnect(&mut self) {
        info!("[ DISCONNECTING ]");
        self.online = false;
    }

    /// Makes the storage go online.
    /// It will send and receive transactions.
    pub fn connect(&mut self) {
        info!("[ CONNECTING ]");
        self.online = true;
    }

    /// Checks if the storage is online.
    /// If it is not, it will return an error after a timeout.
    pub fn check_online(&self) -> Result<(), String> {
        if self.online {
            Ok(())
        } else {
            thread::sleep(Duration::from_millis(TIMEOUT + TIMEOUT / 10));
            Err("Storage is offline".to_string())
        }
    }

    pub fn coordinate_msg(msg: Message, storage: Arc<Mutex<PointStorage>>) -> Result<TxOk, String> {
        let mut storage = storage.lock().map_err(|_| "Failed to lock storage")?;
        let transaction = Transaction::new(storage.self_address.clone(), &msg)?;

        let servers = storage.get_other_servers();
        let online = storage.online;
        let pending = storage.pending.clone();

        let record_ref = storage.get_point_record(transaction.client_id);
        drop(storage);
        let record = record_ref.lock().map_err(|_| "Failed to lock record")?;

        record.wait_die(&transaction)?;

        let points = record.points.clone();
        let mut points = points.lock().map_err(|_| "Failed to lock points")?;
        drop(record);

        let result = points.coordinate(transaction, servers, online, pending);
        drop(points);

        let mut record = record_ref.lock().map_err(|_| "Failed to lock record")?;
        record.transaction = None;

        result
    }

    pub fn coordinate_tx(
        transaction: Transaction,
        storage: Arc<Mutex<PointStorage>>,
    ) -> Result<TxOk, String> {
        let mut storage = storage.lock().map_err(|_| "Failed to lock storage")?;

        let servers = storage.get_other_servers();
        let online = storage.online;
        let pending = storage.pending.clone();

        let record_ref = storage.get_point_record(transaction.client_id);
        drop(storage);
        let record = record_ref.lock().map_err(|_| "Failed to lock record")?;

        record.wait_die(&transaction)?;

        let points = record.points.clone();
        let mut points = points.lock().map_err(|_| "Failed to lock points")?;
        drop(record);

        let result = points.coordinate(transaction, servers, online, pending);
        drop(points);

        let mut record = record_ref.lock().map_err(|_| "Failed to lock record")?;
        record.transaction = None;

        result
    }

    pub fn set_on_connect(storage: Arc<Mutex<Self>>) {
        let lock = storage.clone();
        let lock = lock.lock().unwrap();

        let pending = lock.pending.clone();
        pending.set_on_connect(Box::new(move || {
            let storage = storage.clone();
            Self::on_connect(storage)
        }))
    }
    pub fn on_connect(storage: Arc<Mutex<Self>>) {
        let mut storage = storage.lock().unwrap();
        let servers = storage.get_other_servers();

        // At least half the servers must be online
        for addr in servers {
            let points = sync_with(&addr);
            if let Ok(points) = points {
                storage.points = points;
                return;
            }
        }
        error!("Failed to sync with any server on connect. This should not happen!");
    }
}

/*#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}*/
