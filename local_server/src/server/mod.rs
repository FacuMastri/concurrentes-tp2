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

use crate::server::message::{receive_from, respond_to, SyncRequest};

use self::{
    message::{ConnectReq, CONNECT, SYNC, TRANSACTION},
    transaction::Transaction,
};

#[derive(Debug)]
/// A server that listens for incoming connections and handles them.
/// It is responsible for receiving and sending messages to clients.
/// It is also responsible for storing the points.
/// It is also responsible for synchronizing the points with other servers.
pub struct Server {
    address: String,
    listener: TcpListener,
    handlers: Vec<JoinHandle<()>>,
    points: Arc<Mutex<PointStorage>>,
}

impl Server {
    /// Creates a new server that listens on the given address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to listen on.
    /// * `core_server_addr` - The address of any known server.
    pub fn new(address: String, core_server_addr: Option<String>) -> Server {
        let listener = TcpListener::bind(address.clone()).unwrap();

        Server {
            address: address.clone(),
            listener,
            handlers: vec![],
            points: PointStorage::new(address, core_server_addr),
        }
    }

    /// Starts listening for incoming connections spawning a new thread to listen for each connection.
    pub fn listen(mut self) -> JoinHandle<()> {
        let listener = self.listener.try_clone().unwrap();

        thread::spawn(move || {
            debug!("Listening on {}", self.address);
            for stream in listener.incoming() {
                let new_connection = stream.unwrap();
                self.handle_stream(new_connection);
            }
        })
    }

    /// Handles a stream by spawning a new thread to handle it.
    /// The new connection could be a client (coffee machine) or another server.
    fn handle_stream(&mut self, mut stream: TcpStream) {
        let mut type_buf = [0; 1];

        stream
            .read_exact(&mut type_buf)
            .unwrap_or_else(|_| error!("Could not read from stream"));

        match type_buf[0] {
            CLIENT_CONNECTION => self.spawn_client_connection_handler(stream),
            SERVER_MESSAGE => self.spawn_server_message_handler(stream),
            _ => error!("Unknown message type"),
        }
    }

    /// Spawns a new thread to handle a client connection.
    fn spawn_client_connection_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();
        let handler = thread::spawn(move || {
            Self::connection_handler(stream, points);
        });

        self.handlers.push(handler);
    }

    /// Handles messages from a client connection while the connection is open.
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

    /// Handles a message from a client connection.
    /// The message could mean the beginning of a new transaction.
    /// The message is responded to with a message containing the points that were affected by the transaction.
    /// The points are also synchronized with other servers.
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

    /// Handles a message from a client connection that needs to be distributed to other servers.
    /// Verifies if the transaction could be completed and attempts to distribute it.
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

    /// Spawns a new thread to handle a received message from another server.
    fn spawn_server_message_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();
        let handler = thread::spawn(move || {
            Self::server_message_handler(stream, points);
        });

        self.handlers.push(handler);
    }

    /// Handles the received message from another server.
    /// The message could be a request to synchronize points, a new transaction or a connection request.
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

    /// Handles a connection request from another server.
    /// The connection request is responded to with a message containing the list of all available servers.
    fn handle_server_connection(
        mut stream: TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive_from(&mut stream)?;

        let request: ConnectReq =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse connect req")?;

        let mut points = points.lock().unwrap();

        debug!("Connect {:?}", request.addr);
        let res = points.add_connection(request)?;

        respond_to(&mut stream, res)
    }

    /// Handles a synchronization request from another server.
    /// The synchronization request is responded to with a message containing the points for each client.
    fn handle_server_sync(
        mut stream: TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive_from(&mut stream)?;

        let req: SyncRequest =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse connect req")?;

        let points = points.lock().unwrap();

        debug!("Send Sync");
        let res = points.sync(req)?;

        respond_to(&mut stream, res)
    }

    /// Handles a transaction from another server.
    fn handle_server_transaction(
        mut stream: TcpStream,
        points: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive_from(&mut stream)?;

        let tx: Transaction =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse transaction")?;
        debug!("Received: {:?}", tx);

        PointStorage::handle_transaction(points, tx, stream)
    }
}
