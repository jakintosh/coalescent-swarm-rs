use futures::{future, StreamExt};
use std::net::SocketAddr;
use tokio::{
    net::{TcpListener, TcpSocket},
    sync::mpsc::{self},
};
use tokio_tungstenite::{client_async, tungstenite::Message};

use crate::networking::{self, Connection, ConnectionError};

impl Into<ConnectionError> for tokio_tungstenite::tungstenite::Error {
    fn into(self) -> ConnectionError {
        ConnectionError::WebSocketHandshakeFailure
    }
}

impl From<networking::Response> for Message {
    fn from(response: networking::Response) -> Self {
        Message::Text(String::from("message"))
    }
}
impl From<Message> for networking::Message {
    fn from(message: Message) -> Self {
        match message {
            Message::Text(text) => {
                println!("Received text: {}", text);
                networking::Message::Empty
            }
            Message::Binary(_) => {
                println!("Received some binary");
                networking::Message::Empty
            }
            Message::Ping(_) => networking::Message::Empty,
            Message::Pong(_) => networking::Message::Empty,
            Message::Close(_) => networking::Message::Empty,
        }
    }
}

pub struct Client;
impl Client {
    pub async fn connect_local(port: u16) -> Result<(), ConnectionError> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        Client::connect(address).await?;
        Ok(())
    }
    pub async fn connect(address: SocketAddr) -> Result<(), ConnectionError> {
        let uri = Client::get_uri(address);
        let socket = match TcpSocket::new_v4() {
            Ok(sock) => sock,
            Err(_) => return Err(ConnectionError::WebSocketHandshakeFailure),
        };
        let stream = match socket.connect(address).await {
            Ok(strm) => strm,
            Err(_) => return Err(ConnectionError::WebSocketHandshakeFailure),
        };
        let (websocket, _http_response) = match client_async(uri, stream).await {
            Ok(ws) => ws,
            Err(_) => return Err(ConnectionError::WebSocketHandshakeFailure),
        };

        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (ws_sink, ws_stream) = websocket.split();
        let connection = Connection {
            protocol: networking::Protocol::WebSocket,
            address: address,
            stream: ws_stream,
            sink: ws_sink,
            request_tx: request_tx,
        };
        let connection_task = tokio::spawn(Connection::handle_connection(connection));
        let receive_task = tokio::spawn(networking::handle_requests(request_rx));

        future::select(receive_task, connection_task).await;

        Ok(())
    }

    fn get_uri(address: SocketAddr) -> String {
        format!(
            "ws://{address}:{port}",
            address = address.ip().to_string(),
            port = address.port()
        )
    }
}

pub struct Server;
impl Server {
    pub async fn listen_local(port: u16) -> Result<(), ConnectionError> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        Server::listen(address).await?;
        Ok(())
    }
    pub async fn listen(address: SocketAddr) -> Result<(), ConnectionError> {
        let tcp_listener = match TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(_) => return Err(ConnectionError::WebSocketHandshakeFailure),
        };
        println!("ws:// listening on: {}", address.to_string());

        // create transmit/receive async channels
        let (request_tx, request_rx) = mpsc::unbounded_channel();

        // create listening task
        let listen_task = tokio::spawn(Server::listen_for_connections(tcp_listener, request_tx));

        // create receiving task
        let receive_task = tokio::spawn(networking::handle_requests(request_rx));

        future::select(listen_task, receive_task).await;

        Ok(())
    }

    async fn listen_for_connections(
        tcp_listener: TcpListener,
        request_tx: networking::RequestTx,
    ) -> Result<(), std::io::Error> {
        // accept incoming connections until tcp listener closes
        while let Ok((tcp_stream, address)) = tcp_listener.accept().await {
            println!("Incoming TCP connection from: {}", address);

            let request_tx_clone = request_tx.clone();
            tokio::spawn(async move {
                let websocket = match tokio_tungstenite::accept_async(tcp_stream).await {
                    Ok(ws) => ws,
                    Err(_) => {
                        println!("ws:// connection failed with: {}", address);
                        return;
                    }
                };
                println!("ws:// opened with: {}", address);
                let (ws_sink, ws_stream) = websocket.split();
                let connection = Connection {
                    protocol: networking::Protocol::WebSocket,
                    address: address,
                    stream: ws_stream,
                    sink: ws_sink,
                    request_tx: request_tx_clone,
                };
                if let Err(_) = Connection::handle_connection(connection).await {
                    println!("ws:// connection errored with: {}", address);
                    return;
                }
            });
        }
        Ok(())
    }
}
