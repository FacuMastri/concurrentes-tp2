use crate::Order;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    LockOrder(Order),
    FreeOrder(Order),
    CommitOrder(Order),
}

impl From<Message> for [u8; 11] {
    fn from(message: Message) -> Self {
        let mut buf = [0; 11];

        match message {
            Message::LockOrder(order) => {
                buf[0] = 1;
                let order: [u8; 10] = order.into();
                buf[1..(10 + 1)].copy_from_slice(&order[..10]);
            }
            Message::FreeOrder(order) => {
                buf[0] = 2;
                let order: [u8; 10] = order.into();
                buf[1..(10 + 1)].copy_from_slice(&order[..10]);
            }
            Message::CommitOrder(order) => {
                buf[0] = 3;
                let order: [u8; 10] = order.into();
                buf[1..(10 + 1)].copy_from_slice(&order[..10]);
            }
        }

        buf
    }
}

impl From<[u8; 11]> for Message {
    fn from(buf: [u8; 11]) -> Self {
        let mut order_buf = [0; 10];
        order_buf[..10].copy_from_slice(&buf[1..(10 + 1)]);

        let order = Order::from(order_buf);

        match buf[0] {
            1 => Message::LockOrder(order),
            2 => Message::FreeOrder(order),
            3 => Message::CommitOrder(order),
            _ => panic!("Invalid message"),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::OrderAction;

    use super::*;

    fn test_message(message: Message) {
        let buf: [u8; 11] = message.clone().into();
        let message2 = Message::from(buf);
        assert_eq!(message, message2);
    }

    #[test]
    fn lock_order() {
        let order = Order::new("John".to_string(), OrderAction::UsePoints(123));
        let message = Message::LockOrder(order);
        test_message(message);
    }

    #[test]
    fn free_order() {
        let order = Order::new("John".to_string(), OrderAction::UsePoints(123));
        let message = Message::FreeOrder(order);
        test_message(message);
    }

    #[test]
    fn commit_order() {
        let order = Order::new("John".to_string(), OrderAction::UsePoints(123));
        let message = Message::CommitOrder(order);
        test_message(message);
    }
}
