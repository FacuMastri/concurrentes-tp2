use crate::transaction_state::TransactionState;

#[derive(Debug)]
pub struct Message {
    pub transaction_state: TransactionState,
    pub transaction_id: u64,
    pub order_type: u8,
    pub client_id: u64,
    pub points: u64,
}

impl Message {
    pub fn new(transaction_id: u64, order_type: u8, client_id: u64, points: u64) -> Self {
        Message {
            transaction_state: TransactionState::Prepare, // si la transaccion es nueva empieza en estado prepare
            transaction_id,
            order_type,
            client_id,
            points,
        }
    }
}

pub fn be_byte_buffer_to_u64(buffer: &[u8]) -> u64 {
    let mut mask_buffer = [0u8; 8];
    mask_buffer.copy_from_slice(&buffer[0..8]);
    u64::from_be_bytes(mask_buffer)
}

// se entiende en big endian
impl From<Vec<u8>> for Message {
    fn from(v: Vec<u8>) -> Self {
        Message {
            transaction_state: v[0].into(),
            transaction_id: be_byte_buffer_to_u64(&v[1..9]),
            order_type: v[9],
            client_id: be_byte_buffer_to_u64(&v[10..18]),
            points: be_byte_buffer_to_u64(&v[18..]),
        }
    }
}

impl From<Message> for Vec<u8> {
    fn from(data: Message) -> Self {
        let mut res = vec![data.transaction_state.into()];
        res.extend_from_slice(&data.transaction_id.to_be_bytes());
        res.extend_from_slice(&data.order_type.to_be_bytes());
        res.extend_from_slice(&data.client_id.to_be_bytes());
        res.extend_from_slice(&data.points.to_be_bytes());
        res
    }
}

mod tests {

    #[test]
    fn test_message() {
        use crate::message::Message;
        let message = Message::new(12, 1, 98, 100);
        println!("Message: {:?}", message);
        let bytes: Vec<u8> = message.into();
        println!("{:?}", bytes);
        let message: Message = bytes.into();
        println!("Message: {:?}", message);
    }
}
