mod point_storage;

use point_storage::Points;
use points::Message;

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

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
            println!("Listening on {}", self.address);
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
        println!("Connection established with {}", addr);

        let mut buf = [0; 11];

        while stream.read_exact(&mut buf).is_ok() {
            let msg = buf.into();
            Self::handle_message(msg, &mut stream, points.clone())
        }

        println!("Connection closed with {}", addr);
    }

    fn handle_message(msg: Message, stream: &mut TcpStream, _points: Arc<Mutex<Points>>) {
        let res = match msg {
            Message::LockOrder(order) => {
                println!("Lock: {:?}", order);
                stream.write_all(&[1])
            }
            Message::FreeOrder(order) => {
                println!("Free: {:?}", order);
                stream.write_all(&[2])
            }
            Message::CommitOrder(order) => {
                println!("Commit: {:?}", order);
                stream.write_all(&[3])
            }
        };
        if res.is_err() {
            println!("Error while sending response");
        }
    }
}
