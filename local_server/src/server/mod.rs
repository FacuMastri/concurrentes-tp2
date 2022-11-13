mod point_storage;

use point_storage::Points;
use points::{Message, MESSAGE_BUFFER_SIZE};

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use tracing::{debug, error};

#[derive(Debug)]
pub struct Server {
    address: String,
    listener: TcpListener,
    handlers: Vec<JoinHandle<()>>,
    points: Arc<Mutex<Points>>,
}

impl Server {
    pub fn new(address: String) -> Server {
        let listener = TcpListener::bind(address.clone()).unwrap();

        Server {
            address,
            listener,
            handlers: vec![],
            points: Arc::new(Mutex::new(Points::new())),
        }
    }

    pub fn listen(mut self) -> JoinHandle<()> {
        let listener = self.listener.try_clone().unwrap();

        thread::spawn(move || {
            debug!("Listening on {}", self.address);
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                self.spawn_connection_handler(stream);
            }
        })
    }

    fn spawn_connection_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();
        let handler = thread::spawn(move || {
            Self::connection_handler(stream, points);
        });

        self.handlers.push(handler);
    }

    fn connection_handler(mut stream: TcpStream, points: Arc<Mutex<Points>>) {
        let addr = stream.local_addr().unwrap().ip().to_string();
        debug!("Connection established with {}", addr);

        let mut message_buffer = [0; MESSAGE_BUFFER_SIZE];

        while stream.read_exact(&mut message_buffer).is_ok() {
            let msg = message_buffer.into();
            Self::handle_message(msg, &mut stream, points.clone())
        }

        debug!("Connection closed with {}", addr);
    }

    fn handle_message(msg: Message, stream: &mut TcpStream, points: Arc<Mutex<Points>>) {
        let mut points = points.lock().expect("Failed to lock points");
        let result = match msg {
            Message::LockOrder(order) => {
                debug!("Lock: {:?}", order);
                points.lock_order(order)
            }
            Message::FreeOrder(order) => {
                debug!("Free: {:?}", order);
                points.free_order(order)
            }
            Message::CommitOrder(order) => {
                debug!("Commit: {:?}", order);
                points.commit_order(order)
            }
        };

        let response = match result {
            Ok(()) => stream.write_all(&[1]),
            Err(_) => stream.write_all(&[0]),
        };

        if response.is_err() {
            error!("Failed to send response");
        };
    }
}
