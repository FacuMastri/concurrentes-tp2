use std::{thread, time::Duration};

use super::*;
use actix::prelude::*;
use rand::Rng;

const SUCCESS_CHANCE: f64 = 0.5;
const ORDER_MILLIS: u64 = 1000;

pub struct OrderHandler {
    pub point_storage: Addr<PointStorage>,
}

impl Actor for OrderHandler {
    type Context = SyncContext<Self>;
}

impl Handler<HandleOrder> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: HandleOrder, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let order = msg.0;

        // How do I make this async?
        // Should this be transaction-like?

        match order.action {
            OrderAction::UsePoints(points) => {
                self.point_storage.send(UsePoints(points));
            }
            OrderAction::FillPoints(points) => {
                self.point_storage.send(FillPoints(points));
            }
        }

        thread::sleep(Duration::from_millis(ORDER_MILLIS));
        // random with SUCCESS_CHANCE chance
        let success = rand::thread_rng().gen_bool(SUCCESS_CHANCE);

        if success {
            println!("Order completed: {:?}", order);
            // commit transaction
        } else {
            println!("Order failed: {:?}", order);
            // abort transaction
        }
    }
}

impl Handler<WaitStop> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: WaitStop, _ctx: &mut SyncContext<Self>) -> Self::Result {
        match msg.0 {
            Some(barrier) => {
                println!("Waiting for stop signal...");
                barrier.wait();
                println!("Done");
            }
            None => {
                println!("Done");
            }
        }
    }
}
