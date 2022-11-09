use std::{io::Write, net::TcpStream, thread, time::Duration};

use super::*;
use actix::prelude::*;

pub struct PointStorage {
    local_server: TcpStream, // local_points
                             //HashMap< id, points >
}

impl Actor for PointStorage {
    type Context = SyncContext<Self>;
}

impl PointStorage {
    pub fn new(local_server_addr: String) -> Result<Self, String> {
        let mut local_server =
            TcpStream::connect(local_server_addr).or(Err("Could not connect to local server"))?;

        let buf: [u8; 1] = [7];
        local_server.write(&buf).unwrap();
        println!("CONNECTED TO LOCAL SERVER");

        Ok(PointStorage { local_server })
    }
}

impl Handler<LockOrder> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LockOrder, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = match msg.0.action {
            OrderAction::FillPoints(p) => p,
            OrderAction::UsePoints(p) => p,
        };
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}

impl Handler<FreeOrder> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: FreeOrder, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = match msg.0.action {
            OrderAction::FillPoints(p) => p,
            OrderAction::UsePoints(p) => p,
        };
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}

impl Handler<CommitOrder> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: CommitOrder, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = match msg.0.action {
            OrderAction::FillPoints(p) => p,
            OrderAction::UsePoints(p) => p,
        };
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}
