#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderAction {
    UsePoints(usize),
    FillPoints(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub customer_name: String,
    pub action: OrderAction,
}

impl Order {
    pub fn new(customer_name: String, action: OrderAction) -> Self {
        Order {
            customer_name,
            action,
        }
    }

    pub fn parse(string: String) -> Result<Self, String> {
        let mut parts = string.split(',');

        let customer_name = parts.next().unwrap().to_string();
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

        Ok(Order::new(customer_name, action))
    }
}

// TODO: better use of bytes
impl From<Order> for [u8; 10] {
    fn from(order: Order) -> Self {
        let mut buf = [0; 10];

        let customer_name = order.customer_name.as_bytes();
        for i in 0..6 {
            buf[i] = if i < customer_name.len() {
                customer_name[i]
            } else {
                0
            };
        }

        match order.action {
            OrderAction::UsePoints(points) => {
                buf[6] = 1;
                buf[7] = (points / 100) as u8;
                buf[8] = ((points % 100) / 10) as u8;
                buf[9] = (points % 10) as u8;
            }
            OrderAction::FillPoints(points) => {
                buf[6] = 2;
                buf[7] = (points / 100) as u8;
                buf[8] = ((points % 100) / 10) as u8;
                buf[9] = (points % 10) as u8;
            }
        }

        buf
    }
}

impl From<[u8; 10]> for Order {
    fn from(buf: [u8; 10]) -> Self {
        // first 6 bytes are customer name
        // next byte is action type
        // last 3 bytes are points
        let mut customer_name = String::new();
        for char in buf.iter().take(6) {
            if *char != 0 {
                customer_name.push(*char as char);
            }
        }

        let points = (buf[7] as usize) * 100 + (buf[8] as usize) * 10 + (buf[9] as usize);

        let action = match buf[6] {
            1 => OrderAction::UsePoints(points),
            2 => OrderAction::FillPoints(points),
            _ => panic!("Invalid action type"),
        };

        Order::new(customer_name, action)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn test_order(order: Order) {
        let buf: [u8; 10] = order.clone().into();
        let order2 = Order::from(buf);
        assert_eq!(order, order2);
    }

    #[test]
    fn test_order_use() {
        let order = Order::new("John".to_string(), OrderAction::UsePoints(123));
        test_order(order);
    }

    #[test]
    fn test_order_fill() {
        let order = Order::new("John".to_string(), OrderAction::FillPoints(123));
        test_order(order);
    }
}
