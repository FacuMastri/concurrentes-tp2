mod message;
mod point_record;
mod point_storage;
mod transaction;

use point_storage::PointStorage;
use points::{Message, CLIENT_CONNECTION, MESSAGE_BUFFER_SIZE, SERVER_MESSAGE};

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use tracing::{debug, error, info};

use crate::server::message::{receive, respond, SyncReq};

use self::{
    message::{ConnectReq, CONNECT, SYNC, TRANSACTION},
    transaction::Transaction,
};

#[derive(Debug)]
pub struct Server {
    address: String,
    listener: TcpListener,
    handlers: Vec<JoinHandle<()>>,
    points: Arc<Mutex<PointStorage>>,
}

impl Server {
    pub fn new(address: String, core_server_addr: Option<String>) -> Server {
        let listener = TcpListener::bind(address.clone()).unwrap();

        Server {
            address: address.clone(),
            listener,
            handlers: vec![],
            points: PointStorage::new(address, core_server_addr),
        }
    }

    pub fn listen(mut self) -> JoinHandle<()> {
        let listener = self.listener.try_clone().unwrap();

        self.spawn_logger(3000);

        thread::spawn(move || {
            debug!("Listening on {}", self.address);
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                self.handle_stream(stream);
            }
        })
    }

    pub fn spawn_logger(&mut self, interval: u64) {
        let points = self.points.clone();
        let handler = thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(interval));
            let points = points.lock().unwrap();
            // TODO: get a better way to print the points & drop the lock
            // let log = points.log();
            // drop(points);
            // debug!("{}", log);
            debug!("{:#?}", points);
        });
        self.handlers.push(handler);
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

    fn connection_handler(mut stream: TcpStream, points: Arc<Mutex<PointStorage>>) {
        let addr = stream.local_addr().unwrap().ip().to_string();
        debug!("Connection established with {}", addr);

        let mut message_buffer = [0; MESSAGE_BUFFER_SIZE];

        while stream.read_exact(&mut message_buffer).is_ok() {
            let msg = message_buffer.into();
            Self::handle_client_message(msg, &mut stream, points.clone())
        }

        debug!("Connection closed with {}", addr);
    }

    fn handle_client_message(
        msg: Message,
        stream: &mut TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) {
        info!("Received {:?}", msg);

        let result = match msg.handle_locally() {
            Ok(()) => Ok(()),
            Err(_) => Self::handle_client_message_distributively(msg, points),
        };

        let response = u8::from(result.is_ok());

        if stream.write_all(&[response]).is_err() {
            error!("Failed to send response");
        };
        info!("Sent response: {:?} [{}]", result, response);
    }

    fn handle_client_message_distributively(
        msg: Message,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let mut points = points.lock().map_err(|_| "Failed to lock points")?;
        let tx = Transaction::new(points.self_address.clone(), &msg)?;
        let record = points.take_for(&tx)?;
        let mut record = record.lock().map_err(|_| "Failed to lock points")?;
        let servers = points.other_servers();
        drop(points); // q: Are these dropped when returning err ?. a: Yes (copilot says)
        record.coordinate(tx, servers)
    }

    fn spawn_server_message_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();
        let handler = thread::spawn(move || {
            Self::server_message_handler(stream, points);
        });

        self.handlers.push(handler);
    }

    fn server_message_handler(mut stream: TcpStream, points: Arc<Mutex<PointStorage>>) {
        let mut buf = [0; 1];

        stream.read_exact(&mut buf).unwrap();

        let res = match buf[0] {
            CONNECT => Self::handle_server_connection(stream, points),
            SYNC => Self::handle_server_sync(stream, points),
            TRANSACTION => Self::handle_server_transaction(stream, points),
            _ => Err("Unknown message type".to_string()),
        };

        if res.is_err() {
            error!("Failed to handle server message: {:?}", res);
        }
    }

    fn handle_server_connection(
        mut stream: TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive(&mut stream)?;

        let req: ConnectReq =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse connect req")?;

        let mut points = points.lock().unwrap();

        debug!("Connect {:?}", req.addr);
        let res = points.add_connection(req)?;

        respond(&mut stream, res)
    }

    fn handle_server_sync(
        mut stream: TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive(&mut stream)?;

        let req: SyncReq =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse connect req")?;

        let points = points.lock().unwrap();

        debug!("Send Sync");
        let res = points.sync(req)?;

        respond(&mut stream, res)
    }

    fn handle_server_transaction(
        mut stream: TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive(&mut stream)?;

        let tx: Transaction =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse transaction")?;
        debug!("Received: {:?}", tx);

        PointStorage::handle_transaction(points, tx, stream)
    }
}
