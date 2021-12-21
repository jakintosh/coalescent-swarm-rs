/// ===== networking::websocket.rs ============================================
///
/// This module implements a connection::Client and connection::Receiver for
/// the WebSocket protocol. It uses the tungstenite library to implement the
/// WebSocket protocol itself.
use futures::StreamExt;
use std::net::SocketAddr;
use tokio::{
    net::{TcpListener, TcpSocket},
    sync::mpsc::{self},
};
use tokio_tungstenite::{client_async, tungstenite};

use crate::networking::connection::{self, Connection, WireMessage};

/// convert tungstenite websocket error to connection error
impl From<tungstenite::Error> for connection::Error {
    fn from(_: tungstenite::Error) -> connection::Error {
        connection::Error::ProtocolError
    }
}

/// Convert a wire message to a tungstenite websocket message
impl From<WireMessage> for tungstenite::Message {
    fn from(wire_message: WireMessage) -> Self {
        match wire_message {
            WireMessage::Ping(bytes) => tungstenite::Message::Ping(bytes),
            WireMessage::Pong(bytes) => tungstenite::Message::Pong(bytes),
            _ => {
                let bytes = rmp_serde::to_vec_named(&wire_message)
                    .expect("websocket::From<WireMessage> serialization failure");

                tungstenite::Message::Binary(bytes)
            }
        }
    }
}

/// Convert a tungstenite websocket message to a wire message
impl From<tungstenite::Message> for WireMessage {
    fn from(message: tungstenite::Message) -> Self {
        match message {
            tungstenite::Message::Binary(bytes) => {
                rmp_serde::from_read_ref(&bytes).expect("deserialize fail")
            }
            tungstenite::Message::Ping(bytes) => WireMessage::Ping(bytes),
            tungstenite::Message::Pong(bytes) => WireMessage::Pong(bytes),
            _ => WireMessage::Empty,
        }
    }
}

pub async fn connect_localhost(port: u16) -> Result<(), connection::Error> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    connect(address).await?;
    Ok(())
}
pub async fn connect(address: SocketAddr) -> Result<(), connection::Error> {
    let uri = get_ws_uri(address);
    let socket = match TcpSocket::new_v4() {
        Ok(sock) => sock,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    let stream = match socket.connect(address).await {
        Ok(strm) => strm,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    let (websocket, _http_response) = match client_async(uri, stream).await {
        Ok(ws) => ws,
        Err(_) => return Err(connection::Error::ProtocolError),
    };

    let (message_tx, message_rx) = mpsc::unbounded_channel();
    let (ws_sink, ws_stream) = websocket.split();
    let connection = Connection {
        protocol: connection::Protocol::WebSocket,
        address: address,
        sink: ws_sink,
        stream: ws_stream,
        message_tx,
    };
    let connection_task = tokio::spawn(connection::handle_connection(connection));
    let receive_task = tokio::spawn(connection::handle_messages(message_rx));
    match futures::try_join!(receive_task, connection_task) {
        Ok(_) => {}
        Err(conn_error) => println!("websocket::connect error {:?}", conn_error),
    };

    Ok(())
}

fn get_ws_uri(address: SocketAddr) -> String {
    format!(
        "ws://{address}:{port}",
        address = address.ip(),
        port = address.port()
    )
}

pub async fn listen_localhost(port: u16) -> Result<(), connection::Error> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    listen(address).await?;
    Ok(())
}
pub async fn listen(address: SocketAddr) -> Result<(), connection::Error> {
    let tcp_listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(_) => return Err(connection::Error::ProtocolError),
    };
    println!("ws:// listening on: {}", address.to_string());

    // create channels to pass received messages
    let (message_tx, message_rx) = mpsc::unbounded_channel();

    // create and join tasks
    let listen_task = tokio::spawn(accept_connections(tcp_listener, message_tx));
    let receive_task = tokio::spawn(connection::handle_messages(message_rx));
    match futures::try_join!(listen_task, receive_task) {
        Ok(_) => {}
        Err(conn_error) => println!("websocket::listen error {:?}", conn_error),
    };

    Ok(())
}

async fn accept_connections(
    tcp_listener: TcpListener,
    message_tx: connection::MessageTx,
) -> Result<(), connection::Error> {
    // accept incoming connections until tcp listener closes
    while let Ok((tcp_stream, address)) = tcp_listener.accept().await {
        println!("Incoming TCP connection from: {}", address);

        let message_tx_clone = message_tx.clone();
        tokio::spawn(async move {
            let websocket = match tokio_tungstenite::accept_async(tcp_stream).await {
                Ok(ws) => ws,
                Err(_) => {
                    println!("ws://{} connection failed ", address);
                    return;
                }
            };
            println!("ws:// opened with: {}", address);
            let (ws_sink, ws_stream) = websocket.split();
            let connection = Connection {
                protocol: connection::Protocol::WebSocket,
                address: address,
                sink: ws_sink,
                stream: ws_stream,
                message_tx: message_tx_clone,
            };
            if let Err(err) = connection::handle_connection(connection).await {
                println!("ws://{} connection error {{{:?}}}", address, err);
                return;
            }
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::networking::connection::WireMessage;
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn wire_message_to_ws() {
        let bytes = vec![0x0, 0x1, 0x2, 0x3];
        let wire_message = WireMessage::Ping(bytes.clone());
        let message: Message = wire_message.into();
        match message {
            Message::Ping(message_bytes) => {
                assert_eq!(bytes, message_bytes);
                return;
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn ws_message_to_wire() {
        let bytes = vec![0x0, 0x1, 0x2, 0x3];
        let message = Message::Ping(bytes.clone());
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
