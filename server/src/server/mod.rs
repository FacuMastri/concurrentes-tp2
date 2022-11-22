mod message;
mod pending_transactions;
mod ping;
mod point_record;
mod point_storage;
mod transaction;

use point_storage::PointStorage;
use points::{
    ControlBytes, ControlMessage, Message, CLIENT_CONNECTION, CONTROL_MESSAGE, MESSAGE_BUFFER_SIZE,
    SERVER_MESSAGE,
};

use std::thread::JoinHandle;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self},
    time::Duration,
};
use tracing::{debug, error, info, trace};

use crate::server::ping::{ping_to, PingRequest, PingResponse};
use crate::server::{
    message::{receive_from, respond_to, SyncRequest},
    transaction::TransactionAction,
};
use crate::threadpool::{Builder, ThreadPool};

use self::{
    message::{ConnectRequest, CONNECT, PING, SYNC, TRANSACTION},
    transaction::{Transaction, TxOk},
};

#[derive(Debug)]
/// A server that listens for incoming connections and handles them.
/// It is responsible for receiving and sending messages to clients.
/// It is also responsible for storing the points.
/// It is also responsible for synchronizing the points with other servers.
pub struct Server {
    address: String,
    listener: TcpListener,
    points: Arc<Mutex<PointStorage>>,
    thread_pool: ThreadPool,
}

const PING_INTERVAL: u64 = 1000;

const N_THREADS: usize = 10;

const INTERVAL_LOGGER: u64 = 3000;

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
            points: PointStorage::new(address, core_server_addr),
            thread_pool: Builder::new().num_threads(N_THREADS).build(),
        }
    }

    /// Starts listening for incoming connections spawning a new thread to listen for each connection.
    pub fn listen(mut self) -> JoinHandle<()> {
        let listener = self.listener.try_clone().unwrap();

        self.spawn_logger(INTERVAL_LOGGER);
        self.spawn_pending_handler();
        self.spawn_ping_handler();

        thread::spawn(move || {
            debug!("Listening on {}", self.address);
            for stream in listener.incoming() {
                let new_connection = stream.unwrap();
                self.handle_stream(new_connection);
            }
        })
    }

    pub fn spawn_logger(&mut self, interval: u64) {
        let points = self.points.clone();
        self.thread_pool.execute(move || loop {
            thread::sleep(Duration::from_millis(interval));
            let points = points.lock().unwrap();
            debug!("Points: {:?}", points);
        });
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
            CONTROL_MESSAGE => self.handle_control_message(stream),
            _ => error!("Unknown message type"),
        }
    }

    /// Spawns a new thread to handle a client connection.
    fn spawn_client_connection_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();
        self.thread_pool.execute(move || {
            Self::connection_handler(stream, points);
        });
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

        let result = match msg.handle_trivially() {
            Ok(()) => {
                debug!("Handled trivially {:?}", msg);
                Ok(())
            }
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
        PointStorage::coordinate_msg(msg, points)?;
        Ok(())
    }

    /// Spawns a new thread to handle a received message from another server.
    fn spawn_server_message_handler(&mut self, stream: TcpStream) {
        let points = self.points.clone();

        {
            let points = points.lock().expect("Could not lock points");
            if !points.online {
                return;
            }
        }
        self.thread_pool.execute(move || {
            Self::server_message_handler(stream, points);
        });
    }

    /// Handles the received message from another server.
    /// The message could be a request to synchronize points, a new transaction or a connection request.
    fn server_message_handler(mut stream: TcpStream, storage: Arc<Mutex<PointStorage>>) {
        let mut buf = [0; 1];

        stream.read_exact(&mut buf).unwrap();

        let res = match buf[0] {
            CONNECT => Self::handle_server_connection(stream, storage),
            SYNC => Self::handle_server_sync(stream, storage),
            TRANSACTION => Self::handle_server_transaction(stream, storage),
            PING => Self::handle_server_ping(stream, storage),
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

        let request: ConnectRequest =
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
        // debug!("Received: {:?}", tx);
        let action = match tx.action {
            TransactionAction::Add => "ADD",
            TransactionAction::Lock => "LOCK",
            TransactionAction::Free => "FREE",
            TransactionAction::Consume => "CONSUME",
        };
        debug!(
            "Received transaction from coordinator '{}' with timestamp {} for client {} to {} {} points.",
            tx.coordinator, tx.timestamp, tx.client_id, action, tx.points
        );

        PointStorage::handle_transaction(points, tx, stream)
    }

    /// Handles a server control message
    fn handle_control_message(&mut self, mut stream: TcpStream) {
        let mut buf: ControlBytes = ControlMessage::Unknown.into();
        let r = stream.read_exact(&mut buf);

        if r.is_err() {
            error!("Failed to read control message: {:?}", r);
            return;
        }

        let mut points = self.points.lock().expect("Failed to lock points");
        match buf.into() {
            ControlMessage::Disconnect => {
                points.disconnect();
            }
            ControlMessage::Connect => {
                points.connect();
            }
            _ => {}
        }
    }

    /// Spawn a job to handle a pending transactions.
    fn spawn_pending_handler(&mut self) {
        let storage = self.points.clone();
        self.thread_pool.execute(|| {
            Self::pending_handler(storage);
        });
    }

    /// Handles pending transactions.
    /// Coordinates the pending transactions if the server is online.
    fn pending_handler(storage: Arc<Mutex<PointStorage>>) {
        let storage_lock = storage.lock().expect("Failed to lock storage");
        let pending = storage_lock.pending.clone();
        drop(storage_lock);

        loop {
            let storage = storage.clone();
            let transaction = pending.pop().unwrap();
            let op = PointStorage::coordinate_tx(transaction, storage);
            match op {
                Ok(TxOk::Finalized) => {}
                _ => {
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }
    }

    /// Handles a ping request from another server.
    /// The ping request is responded to with an OK message and it is used to check if the other
    /// servers are online or if the current server is online.
    fn handle_server_ping(
        mut stream: TcpStream,
        storage: Arc<Mutex<PointStorage>>,
    ) -> Result<(), String> {
        let res = receive_from(&mut stream)?;
        let points = storage.lock().expect("Failed to lock points");
        let online = points.online;
        if !online {
            return Ok(());
        }

        let _req: PingRequest =
            serde_json::from_slice(&res).map_err(|_| "Failed to parse ping request")?;

        let _res: PingResponse = PingResponse {};

        let serialized_res =
            serde_json::to_string(&_res).expect("Failed to serialize ping response");

        respond_to(&mut stream, serialized_res)
    }

    /// Spawns a job to handle pings to other servers.
    fn spawn_ping_handler(&mut self) {
        let storage = self.points.clone();
        self.thread_pool.execute(move || {
            Self::ping_handler(storage);
        });
    }

    /// Pings to other servers to check if they are online or if the current server is offline.
    /// If no server responded, this server will go into offline mode.
    fn ping_handler(storage: Arc<Mutex<PointStorage>>) {
        loop {
            thread::sleep(Duration::from_millis(PING_INTERVAL));
            let points = storage.lock().expect("Failed to lock points");
            let other_servers = points.get_other_servers();
            let online = points.online;
            let pending = points.pending.clone();
            drop(points);
            let mut ping_response = false;
            for server in other_servers {
                if !online {
                    break;
                }
                if let Ok(_response) = ping_to(&server) {
                    trace!("Ping to {} successful", server);
                    ping_response = true;
                    break;
                } else {
                    trace!("Ping to {} failed", server);
                }
            }
            if ping_response {
                pending.connect();
            } else {
                pending.disconnect();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::server::message::{send_message_to, SyncRequest, SYNC};
    use serde_json::json;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn two_servers_should_sync_with_50_points_on_client_2() {
        let expected_result = json!({
            "points": {
                "2": {
                    "points": [50, 0],
                    "transaction": null,
                }
            }
        })
        .to_string();
        let mut server_1 = Command::new("cargo")
            .args(["run", "--bin", "server", "9000"])
            .spawn()
            .expect("Failed to start server");
        // El sleep es para dar tiempo a buildear al tirar un cargo run
        thread::sleep(Duration::from_millis(1000));
        let mut server_2 = Command::new("cargo")
            .args(["run", "--bin", "server", "9001", "9000"])
            .spawn()
            .expect("Failed to start server");
        thread::sleep(Duration::from_millis(5000));
        let mut coffee_maker = Command::new("cargo")
            .current_dir("../")
            .args([
                "run",
                "--bin",
                "coffee_maker",
                "9000",
                "assets/orders-3.csv",
            ])
            .spawn()
            .expect("Failed to start coffee maker");
        coffee_maker.wait().unwrap();

        let synced_points = send_message_to(SYNC, SyncRequest {}, &"localhost:9000".to_owned())
            .expect("Failed to sync");
        server_1.kill().expect("Failed to kill server 1");
        server_2.kill().expect("Failed to kill server 2");

        assert_eq!(synced_points, expected_result);
    }
}
