use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{Arc, Mutex},
};

use points::{Message, MessageBytes, Order, OrderAction};

type PointMap = HashMap<u16, usize>;

#[derive(Debug)]
pub struct Points {
    points: PointMap,
    to_use: PointMap,
    to_fill: PointMap,
    socket: UdpSocket,
    server: SocketAddr,
}

impl Points {
    pub fn new(_core_server_addr: String) -> Arc<Mutex<Self>> {
        let socket = UdpSocket::bind("localhost:0").expect("Failed to bind UDP socket");

        socket
            .set_read_timeout(Some(std::time::Duration::from_millis(1000)))
            .expect("Failed to set read timeout");

        let servers: Vec<SocketAddr> = "localhost:9015"
            .to_socket_addrs()
            .expect("Invalid Address")
            .collect();
        let server = servers[0];

        println!("Core at {:?}", server);

        Arc::new(Mutex::new(Points {
            points: PointMap::new(),
            to_use: PointMap::new(),
            to_fill: PointMap::new(),
            socket,
            server,
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

    /*fn get_points(point_map: &mut PointMap, client_id: u16) -> Result<usize, ()> {
        if !point_map.contains_key(&client_id) {
            Err(())
        } else {
            let client_points = point_map.get(&client_id).unwrap();
            Ok(*client_points)
        }
    }*/

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

    pub fn handle_message(&mut self, msg: Message) -> Result<(), String> {
        self.send_message(msg.clone())?;

        match msg {
            Message::LockOrder(order) => {
                println!("Lock: {:?}", order);
                self.lock_order(order)
            }
            Message::FreeOrder(order) => {
                println!("Free: {:?}", order);
                self.free_order(order)
            }
            Message::CommitOrder(order) => {
                println!("Commit: {:?}", order);
                self.commit_order(order)
            }
        }
    }

    fn send_message(&self, msg: Message) -> Result<(), String> {
        let msg_bytes: MessageBytes = msg.into();
        self.socket
            .send_to(&msg_bytes, self.server)
            .map_err(|_| "Failed to send message")?;

        let mut buf = [0; 1];
        self.socket
            .recv(&mut buf)
            .map_err(|_| "Failed to receive message")?;

        println!("Received: {:?}", buf);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lock_fill_points() {
        let points = Points::new("".to_string());
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
