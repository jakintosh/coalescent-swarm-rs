#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

use crate::messaging::WireMessage;
use crate::networking::connection;

#[derive(Serialize, Deserialize)]
pub(crate) struct Message {
    sender: SocketAddr,
    message: WireMessage,
}

pub(crate) async fn listen(addr: SocketAddr) -> Result<(), connection::Error> {
    let socket = match UdpSocket::bind(addr).await {
        Ok(s) => s,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    let mut buf = [0; 1200];
    if let Ok((len, addr)) = socket.recv_from(&mut buf).await {
        tokio::spawn(receive_message(buf.clone(), len, addr));
    }

    Ok(())
}

pub(crate) async fn connect(addr: SocketAddr) -> Result<(), connection::Error> {
    Ok(())
}

async fn receive_message(buf: [u8; 1200], len: usize, addr: SocketAddr) {
    let buf = &buf[..len];
    let wire_message: WireMessage = match rmp_serde::from_read_ref(buf) {
        Ok(v) => v,
        Err(_) => todo!(),
    };

    // recv'd wire_message from addr
}
