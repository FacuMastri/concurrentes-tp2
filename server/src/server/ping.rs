use crate::server::message::{send_message_to, PING};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Serialize, Deserialize, Debug)]
pub struct PingRequest;

#[derive(Serialize, Deserialize, Debug)]
pub struct PingResponse;

pub fn ping_to(addr: &String) -> Result<(), String> {
    let msg = PingRequest {};
    debug!("Sending PING to {}", addr);
    let res = send_message_to(PING, msg, addr)?;
    let res: PingResponse = serde_json::from_str(&res).map_err(|_| "Failed to parse response")?;
    debug!("Response received: {:?}", res);
    Ok(())
}
