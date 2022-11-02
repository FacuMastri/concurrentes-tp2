use super::TakeOrders;
use actix::prelude::*;

pub struct OrderTaker {}

impl Actor for OrderTaker {
    type Context = Context<Self>;
}

impl Handler<TakeOrders> for OrderTaker {
    type Result = ();

    fn handle(&mut self, msg: TakeOrders, _ctx: &mut Context<Self>) -> Self::Result {
        println!("OrderTaker received: {}", msg.0);
    }
}
