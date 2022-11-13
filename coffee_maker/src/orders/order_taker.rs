use std::{
    fs::File,
    io::{BufRead, BufReader},
    thread,
    time::Duration,
};

use super::*;
use actix::prelude::*;
use tracing::info;

pub struct OrderTaker {
    pub handler: Addr<OrderHandler>,
}

impl Actor for OrderTaker {
    type Context = SyncContext<Self>;
}

impl Handler<TakeOrders> for OrderTaker {
    type Result = ();

    fn handle(&mut self, msg: TakeOrders, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let file_path = msg.0;
        let file = File::open(file_path).unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            if let Ok(order) = Order::parse(line.unwrap()) {
                info!("Order taken: {:?}", order);
                self.handler.do_send(HandleOrder(order));
                thread::sleep(Duration::from_secs(1));
            }
        }

        info!("Done taking orders");
    }
}
