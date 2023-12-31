mod orders;
use std::sync::{Arc, Barrier};

use actix::prelude::*;
use orders::*;
use points::parse_addr;
use std::process::exit;
use tracing::{error, trace, Level};
use tracing_subscriber::FmtSubscriber;

const DISPENSERS: usize = 3;
const DEFAULT_ORDERS: &str = "../assets/orders.csv";

enum Arguments {
    LocalServer = 1,
    Orders,
    SuccessChance,
}

// Result with any error
type Res = Result<(), Box<dyn std::error::Error>>;

fn parse_args() -> (String, String, f64) {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        return (
            parse_addr(args[Arguments::LocalServer as usize].clone()),
            DEFAULT_ORDERS.to_string(),
            DEFAULT_SUCCESS_CHANCE,
        );
    }
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 {
        return (
            parse_addr(args[Arguments::LocalServer as usize].clone()),
            args[Arguments::Orders as usize].clone(),
            DEFAULT_SUCCESS_CHANCE,
        );
    }
    if args.len() == 4 {
        return (
            parse_addr(args[Arguments::LocalServer as usize].clone()),
            args[Arguments::Orders as usize].clone(),
            args[Arguments::SuccessChance as usize]
                .parse::<f64>()
                .unwrap(),
        );
    }
    error!("Usage: coffee_maker <local_server> [<orders>] [<success_chance>]");
    exit(-1);
}

#[actix_rt::main]
async fn main() -> Res {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let (local_server_addr, orders_path, success_chance) = parse_args();

    let point_storage = SyncArbiter::start(1, move || {
        PointStorage::new(local_server_addr.clone()).unwrap()
    });

    let order_handler = SyncArbiter::start(DISPENSERS, move || OrderHandler {
        point_storage: point_storage.clone(),
        success_chance,
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
        trace!("Sent stop signal to handler {}", i);
    }

    order_handler.send(WaitStop(None)).await?;

    Ok(())
}

#[cfg(test)]
mod tests {}
