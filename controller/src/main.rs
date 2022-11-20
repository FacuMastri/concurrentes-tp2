use std::io::{self, BufRead, Write};

use points::{parse_addr, ControlMessage, CONTROL_MESSAGE};

#[derive(Debug)]
struct Request {
    msg: ControlMessage,
    addr: String,
}

impl Request {
    pub fn parse(line: &str) -> Option<Request> {
        let mut parts = line.split_whitespace();
        let msg = match parts.next() {
            Some(t) => match t.chars().next() {
                Some('D') => ControlMessage::Disconnect,
                Some('d') => ControlMessage::Disconnect,
                Some('C') => ControlMessage::Connect,
                Some('c') => ControlMessage::Connect,
                _ => ControlMessage::Unknown,
            },
            _ => return None,
        };
        let addr = match parts.next() {
            Some(addr) => parse_addr(addr.to_string()),
            None => return None,
        };
        Some(Request { msg, addr })
    }

    pub fn send(self) -> Result<(), std::io::Error> {
        let mut stream = std::net::TcpStream::connect(&self.addr)?;
        let type_byte = [CONTROL_MESSAGE];
        let bytes: [u8; 1] = self.msg.into();
        stream.write_all(&type_byte)?;
        stream.write_all(&bytes)?;
        Ok(())
    }
}

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let request = Request::parse(&line);
        println!("{:?}", request);
        if let Some(request) = request {
            let res = request.send();
            println!("{:?}", res);
        }
    }
}
