mod server;

use server::Server;
fn main() {
    let server = Server::new("localhost:9015".to_string());
    let handler = server.listen();

    handler.join().unwrap();
}

#[cfg(test)]
mod tests {
    //use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(1, 1);
    }
}