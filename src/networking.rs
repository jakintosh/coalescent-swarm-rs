pub use crate::networking::connection::{connect_ws, listen_ws, Connection, ConnectionHandle};

mod connection;
use crate::messaging::{WireProtocol, WireTx};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;

pub fn get_local_ip_address() -> Option<IpAddr> {
    match local_ip_address::local_ip() {
        Ok(ip_address) => Some(ip_address),
        Err(_) => None,
    }
}

pub enum Error {
    SocketError,
    DeserializationError,
}

#[derive(Clone)]
pub struct Node {
    pub addr: SocketAddr,
    pub socket: Arc<UdpSocket>,
    wire_tx: WireTx,
}
impl Node {
    pub async fn new(addr: SocketAddr, wire_tx: WireTx) -> Node {
        let socket = tokio::net::UdpSocket::bind(addr)
            .await
            .expect(&format!("Couldn't bind to UDP socket {}", addr.to_string()));
        println!("udp:// socket bound on {}", addr);

        // wrap in arc for concurrency
        let socket = Arc::new(socket);

        Node {
            addr,
            socket,
            wire_tx,
        }
    }
    pub async fn listen(&mut self) -> Result<(), Error> {
        println!("udp:// listening on {}", self.addr);

        // read incoming messages
        let mut buf = [0u8; 1200];
        while let Ok((len, addr)) = self.socket.recv_from(&mut buf).await {
            let buf = buf.clone();
            let wire_tx = self.wire_tx.clone();
            tokio::spawn(async move {
                let buf = &buf[..len];
                match rmp_serde::from_read_ref(buf) {
                    Ok(wire_msg) => {
                        if let Err(e) = wire_tx.send((addr, wire_msg)) {
                            println!("Couldn't handle UDP packet from {}: {}", addr, e);
                            return;
                        };
                    }
                    Err(err) => {
                        println!("Malformed/Invalid UDP packet from {}: {}", addr, err);
                        return;
                    }
                }
            });
        }

        Ok(())
    }
    pub async fn send(&self, addr: SocketAddr, wire_message: WireProtocol) -> Result<(), Error> {
        let buf = rmp_serde::to_vec_named(&wire_message).expect("Couldn't serialize wire message");
        let sent = self
            .socket
            .send_to(buf.as_ref(), addr)
            .await
            .map_err(|_| Error::SocketError)?;
        println!("Sent {} bytes", sent);

        Ok(())
    }
}
