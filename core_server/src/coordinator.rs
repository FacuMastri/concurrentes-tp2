use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use crate::message::Message;
use crate::TransactionResponse;
use crate::TransactionState;
use std::convert::TryInto;
use tracing::{debug, error};

fn id_to_addr(id: usize) -> String {
    "127.0.0.1:1234".to_owned() + &*id.to_string()
}

const TRANSACTION_COORDINATOR_ADDR: &str = "127.0.0.1:1234";
const STAKEHOLDERS: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(10);

struct TransactionCoordinator {
    log: HashMap<u64, TransactionState>,
    socket: UdpSocket,
    responses: Arc<(Mutex<Vec<Option<TransactionState>>>, Condvar)>,
}

impl TransactionCoordinator {
    fn new() -> Self {
        let ret = TransactionCoordinator {
            log: HashMap::new(),
            socket: UdpSocket::bind(TRANSACTION_COORDINATOR_ADDR).unwrap(),
            responses: Arc::new((Mutex::new(vec![None; STAKEHOLDERS]), Condvar::new())),
        };

        let mut clone = ret.clone();
        thread::spawn(move || clone.responder());

        ret
    }

    fn submit(&mut self, message: Message) -> bool {
        match self.log.get(&message.client_id) {
            None => self.prepare(message),
            Some(TransactionState::Wait) => self.prepare(message),
            Some(TransactionState::Commit) => self.commit(message),
            Some(TransactionState::Abort) => self.abort(message),
            _ => todo!(),
        }
    }

    fn prepare(&mut self, message: Message) -> bool {
        self.log
            .insert(message.transaction_id, TransactionState::Wait);
        debug!("[COORDINATOR] prepare {}", message.transaction_id);
        self.broadcast_and_wait(message, TransactionState::Commit)
    }

    fn commit(&mut self, message: Message) -> bool {
        self.log
            .insert(message.transaction_id, TransactionState::Commit);
        debug!("[COORDINATOR] commit {}", message.transaction_id);
        self.broadcast_and_wait(message, TransactionState::Commit)
    }

    fn abort(&mut self, message: Message) -> bool {
        self.log
            .insert(message.transaction_id, TransactionState::Abort);
        debug!("[COORDINATOR] abort {}", message.transaction_id);
        !self.broadcast_and_wait(message, TransactionState::Abort)
    }

    fn broadcast_and_wait(&self, message: Message, expected: TransactionState) -> bool {
        let transaction_id = message.transaction_id;
        *self.responses.0.lock().unwrap() = vec![None; STAKEHOLDERS];
        let msg: Vec<u8> = message.into();
        for stakeholder in 0..STAKEHOLDERS {
            debug!(
                "[COORDINATOR] Envió {:?} id {} a {}",
                msg, transaction_id, stakeholder
            );
            self.socket.send_to(&msg, id_to_addr(stakeholder)).unwrap();
        }
        let responses = self.responses.1.wait_timeout_while(
            self.responses.0.lock().unwrap(),
            TIMEOUT,
            |responses| responses.iter().any(Option::is_none),
        );
        if responses.is_err() {
            error!("[COORDINATOR] timeout {}", transaction_id);
            false
        } else {
            responses
                .unwrap()
                .0
                .iter()
                .all(|opt| opt.is_some() && opt.unwrap() == expected)
        }
    }

    fn responder(&mut self) {
        loop {
            let mut buf = [0; 26];
            let (_size, _from) = self.socket.recv_from(&mut buf).unwrap();
            let id_from = usize::from_le_bytes(buf[1..].try_into().unwrap());
            let message: Vec<u8> = buf.into();
            let message: TransactionResponse = message.into();

            match message.transaction_state {
                TransactionState::Commit => {
                    debug!("[COORDINATOR] recibí COMMIT de {}", id_from);
                    self.responses.0.lock().unwrap()[id_from] = Some(TransactionState::Commit);
                    self.responses.1.notify_all();
                }
                TransactionState::Abort => {
                    debug!("[COORDINATOR] recibí ABORT de {}", id_from);
                    self.responses.0.lock().unwrap()[id_from] = Some(TransactionState::Abort);
                    self.responses.1.notify_all();
                }
                _ => {
                    debug!("[COORDINATOR] ??? {}", id_from);
                }
            }
        }
    }

    fn clone(&self) -> Self {
        TransactionCoordinator {
            log: HashMap::new(),
            socket: self.socket.try_clone().unwrap(),
            responses: self.responses.clone(),
        }
    }
}
