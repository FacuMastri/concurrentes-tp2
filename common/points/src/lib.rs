mod order;
pub use order::*;

mod message;
pub use message::*;

pub const CLIENT_CONNECTION: u8 = 1;
pub const SERVER_MESSAGE: u8 = 2;
