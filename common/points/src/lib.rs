mod order;
pub use order::*;

mod message;
pub use message::*;

pub const CLIENT_CONNECTION: u8 = 1;
pub const SERVER_MESSAGE: u8 = 2;

pub fn parse_addr(addr_or_port: String) -> String {
    if addr_or_port.contains(':') {
        addr_or_port
    } else {
        format!("localhost:{}", addr_or_port)
    }
}
