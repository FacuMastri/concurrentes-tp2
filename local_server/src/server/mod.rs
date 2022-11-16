mod message;
mod point_storage;

use point_storage::Points;
use points::{Message, CLIENT_CONNECTION, MESSAGE_BUFFER_SIZE, SERVER_MESSAGE};

use std::{
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use tracing::{debug, error};

use self::message::{ConnectReq, CONNECT};

#[derive(Debug)]
pub struct Server {
    address: String,
    listener: TcpListener,
    handlers: Vec<JoinHandle<()>>,
    points: Arc<Mutex<Points>>,
}

impl Server {
    pub fn new(address: String, core_server_addr: Option<String>) -> Server {
        let listener = TcpListener::bind(address.clone()).unwrap();

        Server {
            address: address.clone(),
            listener,
            handlers: vec![],
            points: Points::new(address, core_server_addr),
        }
    }

    pub fn listen(mut self) -> JoinHandle<()> {
        let listener = self.listener.try_clone().unwrap();

        thread::spawn(move || {
            debug!("Listening on {}", self.address);
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                self.handle_stream(stream);
            }
        })
    }

    fn handle_stream(&mut self, mut stream: TcpStream) {
        let mut type_buf = [0; 1];

        stream
            .read_exact(&mut type_buf)
            .unwrap_or_else(|_| error!("Could not read from stream"));

        match type_buf[0] {
            CLIENT_CONNECTION => self.spawn_connection_handler(stream),
            SERVER_MESSAGE => self.spawn_server_message_handler(stream),
            _ => error!("Unknown message type"),
        }
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
            Self::handle_client_message(msg, &mut stream, points.clone())
        }

        debug!("Connection closed with {}", addr);
    }

    fn handle_client_message(msg: Message, stream: &mut TcpStream, points: Arc<Mutex<Points>>) {
        let mut points = points.lock().expect("Failed to lock points");
        let result = points.handle_message(msg);

        let response = match result {
            Ok(()) => stream.write_all(&[1]),
            Err(err) => {
                error!("Error: {}", err);
                stream.write_all(&[0])
            }
        };

        if response.is_err() {
            error!("Failed to send response");
        };
    }

    fn spawn_server_message_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();
        let handler = thread::spawn(move || {
            Self::server_message_handler(stream, points);
        });

        self.handlers.push(handler);
    }

    fn server_message_handler(mut stream: TcpStream, points: Arc<Mutex<Points>>) {
        let mut buf = [0; 1];

        stream.read_exact(&mut buf).unwrap();

        let res = match buf[0] {
            CONNECT => Self::handle_server_connection(stream, points),
            _ => Err("Unknown message type".to_string()),
        };

        if res.is_err() {
            error!("Failed to handle server message: {:?}", res);
        }
    }

    fn handle_server_connection(
        stream: TcpStream,
        points: Arc<Mutex<Points>>,
    ) -> Result<(), String> {
        let mut buf = String::new();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = BufWriter::new(stream.try_clone().unwrap());

        reader
            .read_line(&mut buf)
            .map_err(|_| "Failed to read line")?;
        let req: ConnectReq = serde_json::from_str(&buf).map_err(|_| "Failed to parse json")?;

        let mut points = points.lock().unwrap();

        debug!("Connect {:?}", req.addr);
        let res = points.add_connection(req)?;

        if let Some(res) = res {
            debug!("Responding {:?}", res);
            let res = serde_json::to_string(&res).map_err(|e| e.to_string())?;
            let res = res.as_bytes();

            writer.write_all(res).map_err(|e| e.to_string())?;
            writer.flush().unwrap();
        }
        Ok(())
    }
}
