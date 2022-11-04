use std::{thread, time::Duration};

use super::*;
use actix::prelude::*;

pub struct OrderHandler {}

impl Actor for OrderHandler {
    type Context = SyncContext<Self>;
}

impl Handler<HandleOrder> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: HandleOrder, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let order = msg.0;

        println!("Got: {:?}", order);

        match order.action {
            OrderAction::UsePoints(_) => thread::sleep(Duration::from_secs(1)),
            OrderAction::FillPoints(_) => thread::sleep(Duration::from_secs(5)),
        }

        println!("Order handled: {:?}", order);
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
