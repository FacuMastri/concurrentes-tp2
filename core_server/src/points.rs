use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)] // FIXME
pub struct Points {
    points: HashMap<u64, u64>,
}

#[allow(dead_code)] // FIXME
impl Points {
    pub fn new() -> Self {
        let points = HashMap::new();
        Points { points }
    }

    pub fn add_points(&mut self, client_id: u64, points: u64) {
        *self.points.entry(client_id).or_insert(0) += points;
    }

    pub fn remove_points(&mut self, client_id: u64, required_points: u64) -> Result<u64, ()> {
        if !self.points.contains_key(&client_id) {
            return Err(());
        }
        let actual_points = *self.points.get(&client_id).unwrap();
        if actual_points < required_points {
            return Err(());
        }
        self.points
            .entry(client_id)
            .and_modify(|e| *e -= required_points);
        Ok(required_points)
    }

    pub fn get_points(&self, client_id: u64) -> Result<u64, ()> {
        if !self.points.contains_key(&client_id) {
            Err(())
        } else {
            let client_points = self.points.get(&client_id).unwrap();
            Ok(*client_points)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_points() {
        let mut points = Points::new();
        points.add_points(42, 100);
        assert_eq!(100, points.get_points(42).unwrap());
    }
    #[test]
    fn remove_points() {
        let mut points = Points::new();
        points.add_points(42, 100);
        let points_removed = points.remove_points(42, 30).unwrap();
        assert_eq!(30, points_removed);
        assert_eq!(70, points.get_points(42).unwrap());
    }
}
