use futures::{future, StreamExt};
use std::net::SocketAddr;
use tokio::{
    net::{TcpListener, TcpSocket},
    sync::mpsc::{self},
};
use tokio_tungstenite::{client_async, tungstenite};

use crate::networking::connection::{self, Connection, WireMessage};

impl From<tokio_tungstenite::tungstenite::Error> for connection::Error {
    fn from(_: tokio_tungstenite::tungstenite::Error) -> connection::Error {
        connection::Error::ProtocolError
    }
}

/// Convert a
impl From<WireMessage> for tungstenite::Message {
    fn from(_: WireMessage) -> Self {
        tungstenite::Message::Text(String::from("message"))
    }
}
impl From<tungstenite::Message> for WireMessage {
    fn from(message: tungstenite::Message) -> Self {
        match message {
            tungstenite::Message::Text(text) => {
                println!("Received text: {}", text);
                WireMessage::Empty
            }
            tungstenite::Message::Binary(_) => {
                println!("Received some binary");
                WireMessage::Empty
            }
            tungstenite::Message::Ping(_) => WireMessage::Empty,
            tungstenite::Message::Pong(_) => WireMessage::Empty,
            tungstenite::Message::Close(_) => WireMessage::Empty,
        }
    }
}

pub struct Client;
impl Client {
    pub async fn connect_local(port: u16) -> Result<(), connection::Error> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        Client::connect(address).await?;
        Ok(())
    }
    pub async fn connect(address: SocketAddr) -> Result<(), connection::Error> {
        let uri = Client::get_ws_uri(address);
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

        future::select(receive_task, connection_task).await;

        Ok(())
    }

    fn get_ws_uri(address: SocketAddr) -> String {
        format!(
            "ws://{address}:{port}",
            address = address.ip().to_string(),
            port = address.port()
        )
    }
}

pub struct Server;
impl Server {
    pub async fn listen_local(port: u16) -> Result<(), connection::Error> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        Server::listen(address).await?;
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

        // create listening task
        let listen_task = tokio::spawn(Server::accept_connections(tcp_listener, message_tx));

        // create receiving task
        let receive_task = tokio::spawn(connection::handle_messages(message_rx));

        future::select(listen_task, receive_task).await;

        Ok(())
    }

    async fn accept_connections(
        tcp_listener: TcpListener,
        message_tx: connection::MessageTx,
    ) -> Result<(), std::io::Error> {
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
}
