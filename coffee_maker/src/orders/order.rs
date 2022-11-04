#[derive(Debug)]
pub enum OrderAction {
    UsePoints(usize),
    FillPoints(usize),
}

#[derive(Debug)]
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
