mod orders;
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use actix::prelude::*;
use orders::*;

const DISPENSERS: usize = 3;

#[actix_rt::main]
async fn main() {
    let path = String::from("../assets/orders.csv");

    let order_handler = SyncArbiter::start(DISPENSERS, || OrderHandler {});

    let order_handler_clone = order_handler.clone();
    let order_taker = SyncArbiter::start(1, move || OrderTaker {
        handler: order_handler_clone.clone(),
    });

    order_taker
        .send(TakeOrders(path))
        .await
        .expect("Failed to take orders");

    let stop_barrier = Arc::new(Barrier::new(DISPENSERS));
    for i in 0..DISPENSERS {
        let stop_barrier = stop_barrier.clone();
        order_handler
            .try_send(WaitStop(Some(stop_barrier)))
            .unwrap();
        println!("Sent stop signal to handler {}", i);
    }

    order_handler.send(WaitStop(None)).await.unwrap();
}

#[cfg(test)]
mod tests {}
