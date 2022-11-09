mod orders;
use std::sync::{Arc, Barrier};

use actix::prelude::*;
use orders::*;

const DISPENSERS: usize = 3;
const DEFAULT_ORDERS: &str = "../assets/orders.csv";

// Result with any error
type Res = Result<(), Box<dyn std::error::Error>>;

fn parse_args() -> (String, String) {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        return (args[1].clone(), DEFAULT_ORDERS.to_string());
    }
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 {
        return (args[1].clone(), args[2].clone());
    }
    panic!("Usage: coffee_maker <local_server> [<orders>]");
}

#[actix_rt::main]
async fn main() -> Res {
    let (local_server_addr, orders_path) = parse_args();

    let point_storage = SyncArbiter::start(1, move || {
        PointStorage::new(local_server_addr.clone()).unwrap()
    });

    println!("Starting {} dispensers...", DISPENSERS);

    let order_handler = SyncArbiter::start(DISPENSERS, move || OrderHandler {
        point_storage: point_storage.clone(),
    });

    let order_handler_clone = order_handler.clone();
    let order_taker = SyncArbiter::start(1, move || OrderTaker {
        handler: order_handler_clone.clone(),
    });

    order_taker.send(TakeOrders(orders_path)).await?;

    handle_stop(order_handler, DISPENSERS).await?;

    Ok(())
}

async fn handle_stop(order_handler: Addr<OrderHandler>, threads: usize) -> Res {
    let stop_barrier = Arc::new(Barrier::new(threads));
    for i in 0..threads {
        let stop_barrier = stop_barrier.clone();
        order_handler
            .try_send(WaitStop(Some(stop_barrier)))
            .unwrap();
        println!("Sent stop signal to handler {}", i);
    }

    order_handler.send(WaitStop(None)).await?;

    Ok(())
}

#[cfg(test)]
mod tests {}
