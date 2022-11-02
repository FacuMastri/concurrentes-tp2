use actix::prelude::*;

pub enum Order {
    UsePoints(usize),
    FillPoints(usize),
}

// Order Taker
#[derive(Message)]
#[rtype(result = "()")]
pub struct TakeOrders(pub String);

// Order Store
#[derive(Message)]
#[rtype(result = "()")]
pub struct NoMoreOrders;

#[derive(Message)]
#[rtype(result = "()")]
pub struct StoreOrder(pub Order);

/*
#[derive(Message)]
#[rtype(result = "()")]
pub struct AskOrder(pub Addr<OrderHandler>);
*/

// Order Handler
#[derive(Message)]
#[rtype(result = "()")]
pub struct HandleOrder(pub Order);
