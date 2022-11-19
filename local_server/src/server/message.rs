use std::{
    collections::HashSet,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
};

use points::SERVER_MESSAGE;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::point_storage::PointMap;

const TIMEOUT: u64 = 1000;
pub const CONNECT: u8 = 1;
pub const SYNC: u8 = 2;
pub const TRANSACTION: u8 = 3;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectReq {
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

pub fn send_message_to(msg_type: u8, msg: impl Serialize, addr: &String) -> Result<String, String> {
    let stream = write_message_to(msg_type, msg, addr)?;
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    let mut response = String::new();
    reader.read_line(&mut response).map_err(|e| e.to_string())?;

    Ok(response)
}

pub fn receive_from(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0; 8];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u64::from_be_bytes(len_buf);

    let mut buf = vec![0; len as usize];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;

    Ok(buf)
}

pub fn respond_to(stream: &mut TcpStream, res: String) -> Result<(), String> {
    debug!("Responding {:?}", res);
    stream.write_all(res.as_bytes()).map_err(|e| e.to_string())
}

pub fn connect_to(my_addr: &String, server_addr: &String) -> Result<HashSet<String>, String> {
    if my_addr == server_addr {
        return Err("Cannot connect to self".to_string());
    }

    let msg = ConnectReq {
        addr: my_addr.to_owned(),
        copy: false,
    };
    debug!("Sending CONNECT to {}", server_addr);
    let res = send_message_to(CONNECT, msg, server_addr)?;

    let res: ConnectResponse =
        serde_json::from_str(&res).map_err(|_| "Failed to parse response")?;

    debug!("Response: {:?}", res);

    Ok(res.servers)
}

pub fn spread_connect_to(addr: &String, server_addr: &String) -> Result<(), String> {
    let msg = ConnectReq {
        addr: addr.to_owned(),
        copy: true,
    };
    debug!("Spreading CONNECT to {}", server_addr);
    send_message_to(CONNECT, msg, server_addr)?;

    Ok(())
}

pub fn sync_with(addr: &String) -> Result<PointMap, String> {
    let msg = SyncRequest {};
    debug!("Sending SYNC to {}", addr);
    let res = send_message_to(SYNC, msg, addr)?;

    let res: SyncResponse = serde_json::from_str(&res).map_err(|_| "Failed to parse response")?;

    debug!("Response: {:?}", res);

    Ok(res.points)
}
