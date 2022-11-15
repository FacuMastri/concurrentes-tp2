mod server;

use server::Server;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn parse_args() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        return args[1].clone();
    }
    panic!("Usage: local_server <core_server>");
}

fn init_logger() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn main() {
    init_logger();

    let core_server_addr = parse_args();
    let server = Server::new("localhost:9099".to_string(), core_server_addr);
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
