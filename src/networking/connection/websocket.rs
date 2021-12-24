/// ===== networking::websocket.rs ============================================
///
/// This module implements a connection::Client and connection::Receiver for
/// the WebSocket protocol. It uses the tungstenite library to implement the
/// WebSocket protocol itself.
///
use futures::StreamExt;
use std::net::SocketAddr;
use tokio::{
    net::{TcpListener, TcpSocket},
    sync::mpsc::UnboundedSender,
};
use tokio_tungstenite::{
    client_async,
    tungstenite::{Error as TungError, Message as TungMessage},
};

use crate::networking::connection::{self, Connection, WireMessage};

/// convert tungstenite websocket error to connection error
impl From<TungError> for connection::Error {
    fn from(_: TungError) -> connection::Error {
        connection::Error::ProtocolError
    }
}

/// Convert a wire message to a tungstenite websocket message
impl From<WireMessage> for TungMessage {
    fn from(wire_message: WireMessage) -> Self {
        match wire_message {
            WireMessage::Ping(bytes) => TungMessage::Ping(bytes),
            WireMessage::Pong(bytes) => TungMessage::Pong(bytes),
            _ => {
                let bytes = rmp_serde::to_vec_named(&wire_message)
                    .expect("websocket::From<WireMessage> serialization failure");

                TungMessage::Binary(bytes)
            }
        }
    }
}

/// Convert a tungstenite websocket message to a wire message
impl From<TungMessage> for WireMessage {
    fn from(message: TungMessage) -> Self {
        match message {
            TungMessage::Binary(bytes) => {
                rmp_serde::from_read_ref(&bytes).expect("deserialize fail")
            }
            TungMessage::Ping(bytes) => WireMessage::Ping(bytes),
            TungMessage::Pong(bytes) => WireMessage::Pong(bytes),
            _ => WireMessage::Empty,
        }
    }
}

pub(crate) async fn listen(
    address: SocketAddr,
    connection_tx: UnboundedSender<Connection<TungMessage, TungError>>,
) -> Result<(), connection::Error> {
    let tcp_listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    println!("ws:// listening on: {}", address.to_string());

    while let Ok((tcp_stream, address)) = tcp_listener.accept().await {
        let websocket = match tokio_tungstenite::accept_async(tcp_stream).await {
            Ok(ws) => ws,
            Err(_) => {
                println!("ws://{} connection failed ", address);
                continue;
            }
        };
        println!("ws:// opened with: {}", address);

        let (ws_sink, ws_stream) = websocket.split();
        let connection = Connection {
            sink: Box::new(ws_sink),
            stream: Box::new(ws_stream),
        };
        if let Err(_) = connection_tx.send(connection) {
            // this means the receiving end is closed, oh well
        }
    }

    Ok(())
}

pub(crate) async fn connect(
    address: SocketAddr,
) -> Result<Connection<TungMessage, TungError>, connection::Error> {
    let socket = match TcpSocket::new_v4() {
        Ok(sock) => sock,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    let stream = match socket.connect(address).await {
        Ok(strm) => strm,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    let (websocket, _http_response) = match client_async(get_ws_uri(address), stream).await {
        Ok((ws, resp)) => (ws, resp),
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    let (ws_sink, ws_stream) = websocket.split();
    let connection = Connection {
        sink: Box::new(ws_sink),
        stream: Box::new(ws_stream),
    };

    Ok(connection)
}

fn get_ws_uri(address: SocketAddr) -> String {
    format!(
        "ws://{address}:{port}",
        address = address.ip(),
        port = address.port()
    )
}

#[cfg(test)]
mod tests {

    use crate::networking::connection::WireMessage;
    use tokio_tungstenite::tungstenite::Message as TungMessage;

    #[test]
    fn wire_message_to_ws() {
        let bytes = vec![0x0, 0x1, 0x2, 0x3];
        let wire_message = WireMessage::Ping(bytes.clone());
        let message: TungMessage = wire_message.into();
        match message {
            TungMessage::Ping(message_bytes) => {
                assert_eq!(bytes, message_bytes);
                return;
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn ws_message_to_wire() {
        let bytes = vec![0x0, 0x1, 0x2, 0x3];
        let message = TungMessage::Ping(bytes.clone());
        let wire_message: WireMessage = message.into();
        match wire_message {
            WireMessage::Ping(message_bytes) => {
                assert_eq!(bytes, message_bytes);
                return;
            }
            _ => assert!(false),
        };
    }
}
