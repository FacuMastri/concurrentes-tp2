use std::{
    collections::{HashMap, HashSet},
    io::Write,
    net::TcpStream,
    sync::{Arc, Mutex},
};

use super::{
    message::{connect_to, spread_connect_to, sync_with, ConnectReq, ConnectRes, SyncReq, SyncRes},
    point_record::{PointRecord, Points},
    transaction::{Transaction, TransactionAction, TransactionState},
};
use tracing::{debug, error};

pub type PointMap = HashMap<u16, PointRecord>;

#[derive(Debug)]
pub struct PointStorage {
    pub points: PointMap,
    pub servers: HashSet<String>,
    pub addr: String,
}

impl PointStorage {
    pub fn new(self_addr: String, server_addr: Option<String>) -> Arc<Mutex<Self>> {
        let mut servers = HashSet::new();
        let mut points = PointMap::new();

        if let Some(addr) = server_addr {
            servers = connect_to(&self_addr, &addr).unwrap();
            points = sync_with(&addr).unwrap();
        } else {
            servers.insert(self_addr.clone());
        }

        Arc::new(Mutex::new(PointStorage {
            points,
            servers,
            addr: self_addr,
        }))
    }

    pub fn get_point_record(&mut self, client_id: u16) -> &mut PointRecord {
        self.points
            .entry(client_id)
            .or_insert_with(PointRecord::new)
    }

    /* FIXME: maybe this is not needed
    fn add_points(point_map: &mut PointMap, client_id: u16, points: usize) -> Result<(), String> {
        *point_map.entry(client_id).or_insert(0) += points;
        Ok(())
    }

    fn remove_points(
        point_map: &mut PointMap,
        client_id: u16,
        points: usize,
    ) -> Result<(), String> {
        if !point_map.contains_key(&client_id) {
            return Err("Client not found".to_string());
        }
        let actual_points = point_map.get(&client_id).unwrap();
        if *actual_points < points {
            return Err("Not enough points".to_string());
        }
        point_map.entry(client_id).and_modify(|e| *e -= points);
        Ok(())
    }
    */

    /*  FIXME: delete this
        pub fn lock_order(&mut self, order: Order) -> Result<(), String> {
            let _client_id = order.client_id;
            match order.action {
                OrderAction::FillPoints(_points) => Ok(()),
                OrderAction::UsePoints(_points) => Ok(()),
            }
        }

        pub fn free_order(&mut self, order: Order) -> Result<(), String> {
            let _client_id = order.client_id;
            match order.action {
                OrderAction::FillPoints(_points) => Ok(()),
                OrderAction::UsePoints(_points) => Ok(()),
            }
        }

        pub fn commit_order(&mut self, order: Order) -> Result<(), String> {
            let _client_id = order.client_id;
            match order.action {
                OrderAction::FillPoints(_points) => Ok(()),
                OrderAction::UsePoints(_points) => Ok(()),
            }
        }

        pub fn handle_message(&mut self, msg: ClientMessage) -> Result<(), String> {
            match msg {
                ClientMessage::LockOrder(order) => self.lock_order(order),
                ClientMessage::FreeOrder(order) => self.free_order(order),
                ClientMessage::CommitOrder(order) => self.commit_order(order),
            }
        }
    */

    pub fn add_connection(&mut self, req: ConnectReq) -> Result<String, String> {
        debug!("Adding connection: {:?}", &req.addr);

        if !req.copy {
            self.spread_connection(req.addr.clone())?;
        }

        self.servers.insert(req.addr);

        let res = ConnectRes {
            servers: self.servers.clone(),
        };

        if req.copy {
            Ok(String::from(""))
        } else {
            serde_json::to_string(&res).map_err(|e| e.to_string())
        }
    }
    pub fn sync(&self, _req: SyncReq) -> Result<String, String> {
        let res = SyncRes {
            points: self.points.clone(),
        };
        serde_json::to_string(&res).map_err(|_| "Failed to serialize points".to_string())
    }

    pub fn spread_connection(&mut self, addr: String) -> Result<(), String> {
        for server in &self.servers {
            if server == &addr || server == &self.addr {
                continue;
            }
            if spread_connect_to(&addr, server).is_err() {
                error!("Failed to spread connection to {}", server);
            }
        }

        Ok(())
    }

    /// Check if order can be fulfilled
    /// Return [mutex guard/arc mutex] of the record
    /// implement wait-die
    pub fn take_for(&mut self, tx: &Transaction) -> Result<Arc<Mutex<Points>>, String> {
        let rec = self.get_point_record(tx.client_id);

        // wait-die
        if let Some(etx) = rec.transaction.clone() {
            if tx.older_than(&etx) {
                return Err("Transaction is older than the current one".to_string());
            }
        }

        // FIXME: point_storage is locked while waiting for this lock
        let points = rec.points.clone();
        let points = points.lock().unwrap();

        match tx.action {
            TransactionAction::Add => Ok(()),
            TransactionAction::Lock => {
                if points.0 < tx.points {
                    Err("Not enough points available".to_string())
                } else {
                    Ok(())
                }
            }
            _ => {
                // Free or Consume
                if points.1 < tx.points {
                    Err("Not enough points locked".to_string())
                } else {
                    Ok(())
                }
            }
        }?;

        Ok(rec.points.clone())
    }

    pub fn handle_transaction(
        storage: Arc<Mutex<Self>>,
        tx: Transaction,
        mut coordinator: TcpStream,
    ) -> Result<(), String> {
        let mut points = storage.lock().map_err(|_| "Failed to lock storage")?;
        let record = points.take_for(&tx);

        let state = if record.is_ok() {
            TransactionState::Proceed as u8
        } else {
            TransactionState::Abort as u8
        };
        coordinator.write_all(&[state]).map_err(|e| e.to_string())?;

        let record = record?;
        let mut record = record.lock().map_err(|_| "Failed to lock record")?;
        drop(points); // q: Are these dropped when returning err ?. a: Yes (copilot says)
        record.handle_transaction(tx, coordinator)
    }
}

/*#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}*/
