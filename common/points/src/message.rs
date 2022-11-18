use crate::{Order, OrderAction, ORDER_BUFFER_SIZE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    LockOrder(Order),
    FreeOrder(Order),
    CommitOrder(Order),
}

pub const MESSAGE_BUFFER_SIZE: usize = ORDER_BUFFER_SIZE + 1;
pub type MessageBytes = [u8; MESSAGE_BUFFER_SIZE];

impl From<Message> for MessageBytes {
    fn from(message: Message) -> Self {
        let mut buf = [0; MESSAGE_BUFFER_SIZE];

        match message {
            Message::LockOrder(order) => {
                buf[0] = 1;
                let order: [u8; ORDER_BUFFER_SIZE] = order.into();
                buf[1..(MESSAGE_BUFFER_SIZE)].copy_from_slice(&order[..ORDER_BUFFER_SIZE]);
            }
            Message::FreeOrder(order) => {
                buf[0] = 2;
                let order: [u8; ORDER_BUFFER_SIZE] = order.into();
                buf[1..(MESSAGE_BUFFER_SIZE)].copy_from_slice(&order[..ORDER_BUFFER_SIZE]);
            }
            Message::CommitOrder(order) => {
                buf[0] = 3;
                let order: [u8; 6] = order.into();
                buf[1..(MESSAGE_BUFFER_SIZE)].copy_from_slice(&order[..ORDER_BUFFER_SIZE]);
            }
        }

        buf
    }
}

impl From<MessageBytes> for Message {
    fn from(buf: MessageBytes) -> Self {
        let mut order_buf = [0; ORDER_BUFFER_SIZE];
        order_buf[..6].copy_from_slice(&buf[1..(MESSAGE_BUFFER_SIZE)]);

        let order = Order::from(order_buf);

        match buf[0] {
            1 => Message::LockOrder(order),
            2 => Message::FreeOrder(order),
            3 => Message::CommitOrder(order),
            _ => panic!("Invalid message"),
        }
    }
}

impl Message {
    pub fn handle_locally(&self) -> Result<(), String> {
        let err = "Could not handle message locally".to_string();

        let order = match self {
            Message::LockOrder(order) => Ok(order),
            Message::FreeOrder(order) => Ok(order),
            Message::CommitOrder(_) => Err(err.clone()),
        }?;

        match order.action {
            OrderAction::UsePoints(_) => Err(err),
            OrderAction::FillPoints(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::OrderAction;

    use super::*;

    fn test_message(message: Message) {
        let buf: [u8; 7] = message.clone().into();
        let message2 = Message::from(buf);
        assert_eq!(message, message2);
    }

    #[test]
    fn lock_order() {
        let order = Order::new(1, OrderAction::UsePoints(123));
        let message = Message::LockOrder(order);
        test_message(message);
    }

    #[test]
    fn free_order() {
        let order = Order::new(50, OrderAction::UsePoints(123));
        let message = Message::FreeOrder(order);
        test_message(message);
    }

    #[test]
    fn commit_order() {
        let order = Order::new(30, OrderAction::UsePoints(123));
        let message = Message::CommitOrder(order);
        test_message(message);
    }
}
