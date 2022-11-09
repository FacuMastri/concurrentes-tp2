use std::{thread, time::Duration};

use super::*;
use actix::prelude::*;
use futures::executor::block_on;
use rand::Rng;

const SUCCESS_CHANCE: f64 = 0.5;
const ORDER_MILLIS: u64 = 1000;

pub struct OrderHandler {
    pub point_storage: Addr<PointStorage>,
}

impl Actor for OrderHandler {
    type Context = SyncContext<Self>;
}

impl OrderHandler {
    fn process_order(&self) -> Result<(), String> {
        thread::sleep(Duration::from_millis(ORDER_MILLIS));
        let success = rand::thread_rng().gen_bool(SUCCESS_CHANCE);
        match success {
            true => Ok(()),
            false => Err(String::from("Order failed")),
        }
    }

    async fn lock_points(&self, order: Order) -> Result<(), String> {
        self.point_storage
            .send(LockPoints(order))
            .await
            .or(Err("MailboxError".to_string()))??;
        Ok(())
    }

    async fn free_points(&self, order: Order) -> Result<(), String> {
        self.point_storage
            .send(FreePoints(order))
            .await
            .or(Err("MailboxError".to_string()))??;
        Ok(())
    }

    async fn commit_points(&self, order: Order) -> Result<(), String> {
        self.point_storage
            .send(CommitPoints(order))
            .await
            .or(Err("MailboxError".to_string()))??;
        Ok(())
    }

    async fn handle_order(&mut self, order: Order) -> Result<(), String> {
        self.lock_points(order.clone()).await?;

        if self.process_order().is_err() {
            self.free_points(order).await?;
            Err(String::from("Order failed"))
        } else {
            self.commit_points(order).await
        }
    }
}

impl Handler<HandleOrder> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOrder, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let order = msg.0;
        block_on(self.handle_order(order))
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
