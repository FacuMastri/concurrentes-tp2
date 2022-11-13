//use point_storage::Points;
use points::{Message, MessageBytes, MESSAGE_BUFFER_SIZE};

use std::{
    net::{SocketAddr, UdpSocket},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct Server {
    listener: UdpSocket,
    handlers: Vec<JoinHandle<()>>,
    //points: Arc<Mutex<Points>>,
}

impl Server {
    pub fn new(address: String) -> Server {
        let listener = UdpSocket::bind(&address).expect("Failed to bind socket");

        Server {
            listener,
            handlers: vec![],
            //points: Points::new(),
        }
    }

    pub fn listen(mut self) -> JoinHandle<()> {
        thread::spawn(move || {
            println!("Listening on {}", self.listener.local_addr().unwrap());
            loop {
                self.spawn_handler();
            }
        })
    }

    fn spawn_handler(&mut self) {
        let socket = self.listener.try_clone().expect("Failed to clone socket");

        let bytes = &mut [0; 20];
        //TODO: why would this fail? Should it be handled?
        let recv = socket.recv_from(bytes).expect("Failed to receive data");

        match recv.0 {
            MESSAGE_BUFFER_SIZE => {
                let msg: MessageBytes = bytes[..MESSAGE_BUFFER_SIZE]
                    .try_into()
                    .expect("Failed to convert to array . This should never happen");
                self.handlers
                    .push(Self::spawn_message_handler(msg.into(), socket, recv.1));
            }
            _ => {
                println!("Received invalid message {:?}", bytes);
            }
        }
    }

    fn spawn_message_handler(
        message: Message,
        socket: UdpSocket,
        addr: SocketAddr,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            println!("Handling message: {:?}", message);
            socket
                .send_to(&[1], &addr)
                .expect("Failed to send response");
        })
    }
}
