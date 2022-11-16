use std::{
    collections::HashSet,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
};

use points::SERVER_MESSAGE;
use serde::{Deserialize, Serialize};
use tracing::debug;

const TIMEOUT: u64 = 1000;
pub const CONNECT: u8 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectReq {
    pub addr: String,
    pub copy: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectRes {
    pub servers: HashSet<String>,
}

pub fn send_to(msg_type: u8, msg: impl Serialize, addr: &String) -> Result<String, String> {
    let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream.try_clone().unwrap());

    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(TIMEOUT)))
        .unwrap();

    let msg = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    let msg = msg.as_bytes();
    let msg_len: [u8; 8] = msg.len().to_be_bytes();

    writer
        .write_all(&[SERVER_MESSAGE])
        .map_err(|e| e.to_string())?;
    writer.write_all(&[msg_type]).map_err(|e| e.to_string())?;

    writer.write_all(&msg_len).map_err(|e| e.to_string())?;
    writer.write_all(msg).map_err(|e| e.to_string())?;
    writer.flush().unwrap();

    let mut res = String::new();
    reader.read_line(&mut res).unwrap();

    Ok(res)
}

pub fn receive(stream: &mut TcpStream) -> Vec<u8> {
    let mut len_buf = [0; 8];
    stream.read_exact(&mut len_buf).unwrap();
    let len = u64::from_be_bytes(len_buf);

    let mut buf = vec![0; len as usize];
    stream.read_exact(&mut buf).unwrap();

    buf
}

pub fn respond(stream: &mut TcpStream, res: String) -> Result<(), String> {
    debug!("Responding {:?}", res);
    stream.write_all(&res.as_bytes()).map_err(|e| e.to_string())
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
    let res = send_to(CONNECT, msg, server_addr).unwrap();

    let res: ConnectRes = serde_json::from_str(&res).unwrap();

    debug!("Response: {:?}", res);

    Ok(res.servers)
}

pub fn spread_connect_to(addr: &String, server_addr: &String) -> Result<(), String> {
    let msg = ConnectReq {
        addr: addr.to_owned(),
        copy: true,
    };
    debug!("SPREADING CONNECT TO {}", server_addr);
    send_to(CONNECT, msg, server_addr)?;

    Ok(())
}
