use std::{thread, time::Duration};

use super::*;
use actix::prelude::*;

pub struct PointStorage {
    //HashMap< id, points >
}

impl Actor for PointStorage {
    type Context = SyncContext<Self>;
}

impl Handler<LockPoints> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LockPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = match msg.0.action {
            OrderAction::FillPoints(p) => p,
            OrderAction::UsePoints(p) => p,
        };
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}

impl Handler<FreePoints> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: FreePoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = match msg.0.action {
            OrderAction::FillPoints(p) => p,
            OrderAction::UsePoints(p) => p,
        };
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}

impl Handler<CommitPoints> for PointStorage {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: CommitPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let points = match msg.0.action {
            OrderAction::FillPoints(p) => p,
            OrderAction::UsePoints(p) => p,
        };
        thread::sleep(Duration::from_millis(points as u64));
        Ok(())
    }
}
