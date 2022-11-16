use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use points::{Message as ClientMessage, Order, OrderAction};

use super::message::{connect_to, spread_connect_to, ConnectReq, ConnectRes};
use tracing::{debug, error};

type PointMap = HashMap<u16, usize>;

#[derive(Debug)]
pub struct Points {
    points: PointMap,
    to_use: PointMap,
    to_fill: PointMap,
    servers: HashSet<String>,
    my_addr: String,
}

impl Points {
    pub fn new(self_addr: String, server_addr: Option<String>) -> Arc<Mutex<Self>> {
        let mut servers = HashSet::new();

        if let Some(addr) = server_addr {
            let _servers = connect_to(&self_addr, &addr).unwrap();
            servers.extend(_servers);
        } else {
            servers.insert(self_addr.clone());
        }

        Arc::new(Mutex::new(Points {
            points: PointMap::new(),
            to_use: PointMap::new(),
            to_fill: PointMap::new(),
            servers,
            my_addr: self_addr,
        }))
    }

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

    pub fn lock_order(&mut self, order: Order) -> Result<(), String> {
        let client_id = order.client_id;
        match order.action {
            OrderAction::FillPoints(points) => {
                Points::add_points(&mut self.to_fill, client_id, points)
            }
            OrderAction::UsePoints(points) => {
                Points::remove_points(&mut self.points, client_id, points)?;
                Points::add_points(&mut self.to_use, client_id, points)
            }
        }
    }

    pub fn free_order(&mut self, order: Order) -> Result<(), String> {
        let client_id = order.client_id;
        match order.action {
            OrderAction::FillPoints(points) => {
                Points::remove_points(&mut self.to_fill, client_id, points)
            }
            OrderAction::UsePoints(points) => {
                Points::remove_points(&mut self.to_use, client_id, points)?;
                Points::add_points(&mut self.points, client_id, points)
            }
        }
    }

    pub fn commit_order(&mut self, order: Order) -> Result<(), String> {
        let client_id = order.client_id;
        match order.action {
            OrderAction::FillPoints(points) => {
                Points::remove_points(&mut self.to_fill, client_id, points)?;
                Points::add_points(&mut self.points, client_id, points)
            }
            OrderAction::UsePoints(points) => {
                Points::remove_points(&mut self.to_use, client_id, points)
            }
        }
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> Result<(), String> {
        match msg {
            ClientMessage::LockOrder(order) => self.lock_order(order),
            ClientMessage::FreeOrder(order) => self.free_order(order),
            ClientMessage::CommitOrder(order) => self.commit_order(order),
        }
    }

    pub fn add_connection(&mut self, req: ConnectReq) -> Result<Option<ConnectRes>, String> {
        debug!("Adding connection: {:?}", &req.addr);

        if !req.copy {
            self.spread_connection(req.addr.clone())?;
        }

        self.servers.insert(req.addr);

        let res = ConnectRes {
            servers: self.servers.clone(),
        };

        if req.copy {
            Ok(None)
        } else {
            Ok(Some(res))
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lock_fill_points() {
        let points = Points::new("Me".to_string(), None);
        let mut points = points.lock().unwrap();
        points
            .lock_order(Order {
                client_id: 420,
                action: OrderAction::FillPoints(100),
            })
            .unwrap();
        assert_eq!(100, *points.to_fill.get(&420).unwrap());
    }
}
