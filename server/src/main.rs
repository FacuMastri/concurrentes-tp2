mod server;

use server::Server;
fn main() {
    let server = Server::new("localhost".to_string(), "9000".to_string());
    server.listen();
}

#[cfg(test)]
mod tests {
    //use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(1, 1);
    }
}
