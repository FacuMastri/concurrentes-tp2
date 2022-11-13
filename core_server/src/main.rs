mod coordinator;
mod logger;
mod message;
mod points;
mod transaction_response;
mod transaction_state;

use logger::Logger;
use message::Message;
use points::Points;
use std::collections::HashMap;
use std::net::UdpSocket;
use transaction_response::TransactionResponse;
use transaction_state::TransactionState;

use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use tracing::{debug, Level};
use tracing_subscriber::FmtSubscriber;

fn logger(rx: Receiver<String>) {
    let mut logger = Logger::new("db.log");
    for msg in rx {
        debug!("{}", msg);
        logger.log(msg);
    }
}

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut points = Points::new();
    let sock = UdpSocket::bind("localhost:1234").unwrap();
    let mut log = HashMap::new();
    let (tx, rx) = mpsc::channel();
    let _ = tx.send("Replica inicializada en localhost:1234".to_string());

    let _ = thread::spawn(|| logger(rx));

    loop {
        let mut buf = [0; 26];

        let (_, addr) = sock.recv_from(&mut buf).unwrap();

        let payload_deserialized: Message = buf.to_vec().into();
        let _ = tx.send(format!("payload_deserialized: {:?}", payload_deserialized));

        let transaction_id = payload_deserialized.transaction_id;
        let order_type = payload_deserialized.order_type;
        let client_id = payload_deserialized.client_id;
        let points_required = payload_deserialized.points;

        let response = match payload_deserialized.transaction_state {
            TransactionState::Prepare => {
                let _ = tx.send("TransactionState: Prepare".to_string());
                match log.get(&transaction_id) {
                    Some(TransactionState::Accept) | Some(TransactionState::Commit) => {
                        let _ = tx.send("TransactionResponse: Commit".to_string());
                        TransactionResponse::new(transaction_id, TransactionState::Commit)
                    }
                    Some(TransactionState::Abort) => {
                        let _ = tx.send("TransactionResponse: Abort".to_string());
                        TransactionResponse::new(transaction_id, TransactionState::Abort)
                    }
                    None => {
                        if (points.get_points(client_id).unwrap() > points_required)
                            & (order_type == 0)
                        {
                            log.insert(transaction_id, TransactionState::Accept);
                            let _ = tx.send("TransactionResponse: Commit".to_string());
                            TransactionResponse::new(transaction_id, TransactionState::Commit)
                        } else {
                            log.insert(transaction_id, TransactionState::Abort);
                            let _ = tx.send("FAILED. TransactionResponse: Abort".to_string());
                            TransactionResponse::new(transaction_id, TransactionState::Abort)
                        }
                    }
                    _ => panic!("Invalid transacciont state"),
                }
            }
            TransactionState::Commit => {
                let _ = tx.send("TransactionState: Commit".to_string());
                match log.get(&transaction_id) {
                    Some(TransactionState::Accept) => {
                        log.insert(transaction_id, TransactionState::Commit);
                        if order_type == 0 {
                            points.add_points(client_id, points_required);
                        } else {
                            points.remove_points(client_id, points_required);
                        }
                        let _ = tx.send("TransactionResponse: Commit".to_string());
                        TransactionResponse::new(transaction_id, TransactionState::Commit)
                    }
                    Some(TransactionState::Commit) => {
                        let _ = tx.send("TransactionResponse: Commit".to_string());
                        TransactionResponse::new(transaction_id, TransactionState::Commit)
                    }
                    Some(TransactionState::Abort) | None => {
                        let _ = tx.send("PANIC; TransactionState::Abort cannot be handled by two fase transactionality algorithm".to_string());
                        panic!("This cannot be handled by two fase transactionality algorithm!");
                    }
                    _ => panic!("This cannot be handled by two fase transactionality algorithm!"),
                }
            }
            TransactionState::Abort => {
                let _ = tx.send("TransactionState: Abort".to_string());
                match log.get(&transaction_id) {
                    Some(TransactionState::Accept) => {
                        log.insert(transaction_id, TransactionState::Abort);
                        let _ = tx.send("TransactionResponse: Abort".to_string());
                        TransactionResponse::new(transaction_id, TransactionState::Abort)
                    }
                    Some(TransactionState::Abort) => {
                        let _ = tx.send("TransactionResponse: Abort".to_string());
                        TransactionResponse::new(transaction_id, TransactionState::Abort)
                    }
                    Some(TransactionState::Commit) | None => {
                        debug!("{} {:?}", transaction_id, log.get(&transaction_id));
                        let _ = tx.send("PANIC; TransactionState::Commit cannot be handled by two fase transactionality algorithm".to_string());
                        panic!("This cannot be handled by two fase transactionality algorithm!");
                    }
                    _ => panic!("This cannot be handled by two fase transactionality algorithm!"),
                }
            }
            _ => panic!("TransactionState Unknow"),
        };

        let response_payload: Vec<u8> = response.into();
        let _ = sock.send_to(response_payload.as_slice(), addr);
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(1, 1);
    }
}
