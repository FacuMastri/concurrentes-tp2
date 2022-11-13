use std::collections::HashMap;

use points::{Order, OrderAction};

type PointMap = HashMap<String, usize>;

#[derive(Debug)]
pub struct Points {
    points: PointMap,
    to_use: PointMap,
    to_fill: PointMap,
}

#[allow(dead_code)] // FIXME
impl Points {
    pub fn new() -> Self {
        Points {
            points: PointMap::new(),
            to_use: PointMap::new(),
            to_fill: PointMap::new(),
        }
    }

    fn add_points(
        point_map: &mut PointMap,
        client_id: String,
        points: usize,
    ) -> Result<(), String> {
        *point_map.entry(client_id).or_insert(0) += points;
        Ok(())
    }

    fn remove_points(
        point_map: &mut PointMap,
        client_id: String,
        points: usize,
    ) -> Result<(), String> {
        if !point_map.contains_key(&client_id) {
            return Err("Client not found".to_string());
        }
        let actual_points = point_map.get(&client_id).unwrap();
        if *actual_points < points {
            return Err("Not enough points".to_string());
        }
        point_map.entry(client_id).and_modify(|e| *e -= points);
        Ok(())
    }

    fn get_points(point_map: &mut PointMap, client_id: String) -> Result<usize, ()> {
        if !point_map.contains_key(&client_id) {
            Err(())
        } else {
            let client_points = point_map.get(&client_id).unwrap();
            Ok(*client_points)
        }
    }

    pub fn lock_order(&mut self, order: Order) -> Result<(), String> {
        let name = order.customer_name;
        match order.action {
            OrderAction::FillPoints(points) => Points::add_points(&mut self.to_fill, name, points),
            OrderAction::UsePoints(points) => {
                Points::remove_points(&mut self.points, name.clone(), points)?;
                Points::add_points(&mut self.to_use, name, points)
            }
        }
    }

    pub fn free_order(&mut self, order: Order) -> Result<(), String> {
        let name = order.customer_name;
        match order.action {
            OrderAction::FillPoints(points) => {
                Points::remove_points(&mut self.to_fill, name, points)
            }
            OrderAction::UsePoints(points) => {
                Points::remove_points(&mut self.to_use, name.clone(), points)?;
                Points::add_points(&mut self.points, name, points)
            }
        }
    }

    pub fn commit_order(&mut self, order: Order) -> Result<(), String> {
        let name = order.customer_name;
        match order.action {
            OrderAction::FillPoints(points) => {
                Points::remove_points(&mut self.to_fill, name.clone(), points)?;
                Points::add_points(&mut self.points, name, points)
            }
            OrderAction::UsePoints(points) => Points::remove_points(&mut self.to_use, name, points),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lock_fill_points() {
        let mut points = Points::new();
        points
            .lock_order(Order {
                customer_name: "John".to_string(),
                action: OrderAction::FillPoints(100),
            })
            .unwrap();
        assert_eq!(100, *points.to_fill.get("John").unwrap());
    }
}
