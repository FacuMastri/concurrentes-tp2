#[derive(Debug)]

pub enum ControlMessage {
    Unknown,
    Disconnect,
    Connect,
}

pub type ControlBytes = [u8; 1];

impl From<ControlMessage> for ControlBytes {
    fn from(msg: ControlMessage) -> Self {
        match msg {
            ControlMessage::Unknown => [0],
            ControlMessage::Disconnect => [1],
            ControlMessage::Connect => [2],
        }
    }
}

impl From<ControlBytes> for ControlMessage {
    fn from(bytes: ControlBytes) -> Self {
        match bytes[0] {
            1 => ControlMessage::Disconnect,
            2 => ControlMessage::Connect,
            _ => ControlMessage::Unknown,
        }
    }
}
