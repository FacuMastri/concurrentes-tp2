use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)] // FIXME
pub struct Points {
    points: HashMap<String, i32>,
}

#[allow(dead_code)] // FIXME
impl Points {
    pub fn new() -> Self {
        let points = HashMap::new();
        Points { points }
    }

    pub fn add_points(&mut self, client_id: String, points: i32) {
        *self.points.entry(client_id).or_insert(0) += points;
    }

    pub fn remove_points(&mut self, client_id: String, points: i32) -> Result<i32, ()> {
        if !self.points.contains_key(&client_id) {
            return Err(());
        }
        let actual_points = self.points.get(&client_id).unwrap();
        if actual_points - points < 0 {
            return Err(());
        }
        self.points.entry(client_id).and_modify(|e| *e -= points);
        Ok(points)
    }

    pub fn get_points(&self, client_id: String) -> Result<i32, ()> {
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
        points.add_points("John".to_string(), 100);
        assert_eq!(100, points.get_points("John".to_string()).unwrap());
    }
    #[test]
    fn remove_points() {
        let mut points = Points::new();
        points.add_points("John".to_string(), 100);
        let points_removed = points.remove_points("John".to_string(), 30).unwrap();
        assert_eq!(30, points_removed);
        assert_eq!(70, points.get_points("John".to_string()).unwrap());
    }
}
