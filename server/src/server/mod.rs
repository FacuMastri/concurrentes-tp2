use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct Server {
    address: String,
    listener: TcpListener,
}

impl Server {
    pub fn new(address: String) -> Server {
        let listener = TcpListener::bind(address.clone()).unwrap();

        Server { address, listener }
    }

    pub fn listen(self) -> JoinHandle<()> {
        let listener = self.listener.try_clone().unwrap();

        thread::spawn(move || {
            println!("Listening on {}", self.address);
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                self.handle_connection(stream);
            }
        })
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let addr = stream.local_addr().unwrap().ip().to_string();
        println!("Connection established with {}", addr);

        let mut buf = [0; 1];

        while stream.read_exact(&mut buf).is_ok() {
            println!("Received: {}", buf[0]);
        }
        println!("Connection closed with {}", addr);
    }
}
