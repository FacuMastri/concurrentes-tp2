use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use super::message::{
    connect_to, spread_connect_to, sync_with, ConnectReq, ConnectRes, SyncReq, SyncRes,
};
use tracing::{debug, error};

pub type PointMap = HashMap<u16, usize>;

#[derive(Debug)]
pub struct Points {
    pub points: PointMap,
    servers: HashSet<String>,
    my_addr: String,
}

impl Points {
    pub fn new(self_addr: String, server_addr: Option<String>) -> Arc<Mutex<Self>> {
        let mut servers = HashSet::new();
        let mut points = PointMap::new();

        if let Some(addr) = server_addr {
            servers = connect_to(&self_addr, &addr).unwrap();
            points = sync_with(&addr).unwrap();
        } else {
            servers.insert(self_addr.clone());
        }

        Arc::new(Mutex::new(Points {
            points,
            servers,
            my_addr: self_addr,
        }))
    }

    fn _add_points(point_map: &mut PointMap, client_id: u16, points: usize) -> Result<(), String> {
        *point_map.entry(client_id).or_insert(0) += points;
        Ok(())
    }

    fn _remove_points(
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
            if server == &addr {
                continue;
            }
            if server == &self.my_addr {
                continue;
            }
            if spread_connect_to(&addr, server).is_err() {
                error!("Failed to spread connection to {}", server);
            }
        }

        Ok(())
    }
}

/*#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}*/
