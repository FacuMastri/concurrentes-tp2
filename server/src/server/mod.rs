use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct Server {
    address: String,
    listener: TcpListener,
    handlers: Vec<JoinHandle<()>>,
}

impl Server {
    pub fn new(address: String) -> Server {
        let listener = TcpListener::bind(address.clone()).unwrap();

        Server {
            address,
            listener,
            handlers: vec![],
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
        let handler = thread::spawn(move || {
            Self::connection_handler(stream);
        });

        self.handlers.push(handler);
    }

    fn connection_handler(mut stream: TcpStream) {
        let addr = stream.local_addr().unwrap().ip().to_string();
        println!("Connection established with {}", addr);

        let mut buf = [0; 1];

        while stream.read_exact(&mut buf).is_ok() {
            println!("Received: {}", buf[0]);
        }
        println!("Connection closed with {}", addr);
    }
}
