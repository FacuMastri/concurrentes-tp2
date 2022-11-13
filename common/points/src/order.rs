#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderAction {
    UsePoints(usize),
    FillPoints(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub client_id: u16,
    pub action: OrderAction,
}

impl Order {
    pub fn new(client_id: u16, action: OrderAction) -> Self {
        Order { client_id, action }
    }

    pub fn parse(line: String) -> Result<Self, String> {
        let mut parts = line.split(',');

        let client_id = parts.next().unwrap().to_string();
        let action = parts.next().unwrap().to_string();

        let action = match action.as_str() {
            "USE" => {
                let points = parts.next().unwrap().parse::<usize>().unwrap();
                OrderAction::UsePoints(points)
            }
            "FILL" => {
                let points = parts.next().unwrap().parse::<usize>().unwrap();
                OrderAction::FillPoints(points)
            }
            _ => return Err("Invalid action".to_string()),
        };
        Ok(Order::new(client_id.parse::<u16>().unwrap(), action))
    }
}

pub const ORDER_BUFFER_SIZE: usize = 6;

// TODO: better use of bytes
impl From<Order> for [u8; ORDER_BUFFER_SIZE] {
    fn from(order: Order) -> Self {
        let mut buf = [0; ORDER_BUFFER_SIZE];

        let client_id = order.client_id;
        buf[0] = (client_id >> 8) as u8;
        buf[1] = client_id as u8;

        match order.action {
            OrderAction::UsePoints(points) => {
                buf[2] = 1;
                buf[3] = (points / 100) as u8;
                buf[4] = ((points % 100) / 10) as u8;
                buf[5] = (points % 10) as u8;
            }
            OrderAction::FillPoints(points) => {
                buf[2] = 2;
                buf[3] = (points / 100) as u8;
                buf[4] = ((points % 100) / 10) as u8;
                buf[5] = (points % 10) as u8;
            }
        }

        buf
    }
}

impl From<[u8; ORDER_BUFFER_SIZE]> for Order {
    fn from(buf: [u8; ORDER_BUFFER_SIZE]) -> Self {
        // First 2 bytes are client id
        // Next byte is action type
        // Last 3 bytes are points
        let client_id = ((buf[0] as u16) << 8) | buf[1] as u16;

        let points = (buf[3] as usize) * 100 + (buf[4] as usize) * 10 + (buf[5] as usize);

        let action = match buf[2] {
            1 => OrderAction::UsePoints(points),
            2 => OrderAction::FillPoints(points),
            _ => panic!("Invalid action type"),
        };

        Order::new(client_id, action)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn test_order(order: Order) {
        let order_from_buf: [u8; 6] = order.clone().into();
        let expected_order = Order::from(order_from_buf);
        assert_eq!(order, expected_order);
    }

    #[test]
    fn test_order_use() {
        let order = Order::new(25, OrderAction::UsePoints(123));
        test_order(order);
    }

    #[test]
    fn test_order_fill() {
        let order = Order::new(30, OrderAction::FillPoints(123));
        test_order(order);
    }
}
