use std::{thread, time::Duration};

use super::*;
use actix::prelude::*;

pub struct PointStorage {
    //HashMap< id, points >
}

impl Actor for PointStorage {
    type Context = SyncContext<Self>;
}

impl Handler<FillPoints> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: FillPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = msg.0;
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}

impl Handler<UsePoints> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UsePoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = msg.0;
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}
