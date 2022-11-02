mod orders;
use actix::prelude::*;
use orders::{OrderTaker, TakeOrders};

use util::hello_world;

#[actix_rt::main]
async fn main() {
    // start new actor
    let addr = OrderTaker {}.start();

    // send message and get future for result
    let res = addr.send(TakeOrders(hello_world())).await;

    // print result
    println!("RESULT: {:?}", res);

    // stop system and exit
    System::current().stop();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(hello_world(), "Hello World");
    }
}
