mod server;

use server::Server;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn parse_addr(addr_or_port: String) -> String {
    if addr_or_port.contains(':') {
        addr_or_port
    } else {
        format!("localhost:{}", addr_or_port)
    }
}

fn parse_args() -> (String, Option<String>) {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        return (parse_addr(args[1].clone()), None);
    }
    if args.len() == 3 {
        return (
            parse_addr(args[1].clone()),
            Some(parse_addr(args[2].clone())),
        );
    }
    panic!("Usage: cargo run --bin local_server <address> [<known_server_address>]");
}

fn init_logger() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn main() {
    init_logger();

    let (addr, core_server_addr) = parse_args();
    let server = Server::new(addr, core_server_addr);
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
