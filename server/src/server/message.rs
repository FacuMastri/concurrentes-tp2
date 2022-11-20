use std::{
    collections::HashSet,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
};

use points::SERVER_MESSAGE;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::point_storage::PointMap;

pub const TIMEOUT: u64 = 1000;
pub const CONNECT: u8 = 1;
pub const SYNC: u8 = 2;
pub const TRANSACTION: u8 = 3;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectRequest {
    pub addr: String,
    pub copy: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectResponse {
    pub servers: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SyncRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct SyncResponse {
    pub points: PointMap,
}

/// Sends a message to the given address.
/// The message is serialized and sent as a byte array.
/// The first byte is the message type.
/// The rest of the bytes are the serialized message.
///
/// # Returns
///
/// The stream to the given address.
pub fn write_message_to(
    msg_type: u8,
    msg: impl Serialize,
    addr: &String,
) -> Result<TcpStream, String> {
    let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(stream.try_clone().unwrap());

    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(TIMEOUT)))
        .map_err(|e| e.to_string())?;

    let msg = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    let msg = msg.as_bytes();
    let msg_len: [u8; 8] = msg.len().to_be_bytes();

    writer
        .write_all(&[SERVER_MESSAGE])
        .map_err(|e| e.to_string())?;
    writer.write_all(&[msg_type]).map_err(|e| e.to_string())?;

    writer.write_all(&msg_len).map_err(|e| e.to_string())?;
    writer.write_all(msg).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;

    Ok(stream)
}

/// Sends a message to the given address and waits for a response.
/// The message is serialized and sent as a byte array.
/// The first byte is the message type.
///
/// # Returns
///
/// The response message.
pub fn send_message_to(msg_type: u8, msg: impl Serialize, addr: &String) -> Result<String, String> {
    let stream = write_message_to(msg_type, msg, addr)?;
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    let mut response = String::new();
    reader.read_line(&mut response).map_err(|e| e.to_string())?;

    Ok(response)
}

/// Receives a message from the given stream.
/// The first eight bytes are the message length.
/// The rest of the bytes are the serialized message.
///
/// # Returns
///
/// The message type and the message.
pub fn receive_from(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0; 8];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u64::from_be_bytes(len_buf);

    let mut buf = vec![0; len as usize];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;

    Ok(buf)
}

/// Responds to a message to the given stream.
pub fn respond_to(stream: &mut TcpStream, response: String) -> Result<(), String> {
    debug!("Responding {:?}", response);
    stream
        .write_all(response.as_bytes())
        .map_err(|e| e.to_string())
}

/// Connects to the given address and sends a connect message.
///
/// # Returns
///
/// The response message.
pub fn connect_to(my_addr: &String, target_address: &String) -> Result<HashSet<String>, String> {
    if my_addr == target_address {
        return Err("Cannot connect to self".to_string());
    }

    let msg = ConnectRequest {
        addr: my_addr.to_owned(),
        copy: false,
    };
    debug!("Sending CONNECT to {}", target_address);
    let res = send_message_to(CONNECT, msg, target_address)?;

    let res: ConnectResponse =
        serde_json::from_str(&res).map_err(|_| "Failed to parse response")?;

    debug!("Response: {:?}", res);

    Ok(res.servers)
}

/// Spreads a CONNECT message to the given target address.
pub fn spread_connect_to(addr: &String, target_address: &String) -> Result<(), String> {
    let msg = ConnectRequest {
        addr: addr.to_owned(),
        copy: true,
    };
    debug!("Spreading CONNECT to {}", target_address);
    send_message_to(CONNECT, msg, target_address)?;

    Ok(())
}

/// Sends a SYNC message to the given address.
///
/// # Returns
///
/// The response message containing the points.
pub fn sync_with(addr: &String) -> Result<PointMap, String> {
    let msg = SyncRequest {};
    debug!("Sending SYNC to {}", addr);
    let res = send_message_to(SYNC, msg, addr)?;

    let res: SyncResponse = serde_json::from_str(&res).map_err(|_| "Failed to parse response")?;

    debug!("Response: {:?}", res);

    Ok(res.points)
}
