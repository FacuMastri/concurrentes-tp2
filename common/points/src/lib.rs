mod order;
pub use order::*;

mod message;
pub use message::*;

mod control;
pub use control::*;

pub const CLIENT_CONNECTION: u8 = 1;
pub const SERVER_MESSAGE: u8 = 2;
pub const CONTROL_MESSAGE: u8 = 3;

pub fn parse_addr(addr_or_port: String) -> String {
    if addr_or_port.contains(':') {
        addr_or_port
    } else {
        format!("localhost:{}", addr_or_port)
    }
}
