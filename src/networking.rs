use futures::{future, SinkExt, StreamExt};
use std::net::{IpAddr, SocketAddr};
use tokio::{
    net::{TcpListener, TcpSocket, TcpStream},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{client_async, tungstenite::Message, WebSocketStream};

type RequestTx = UnboundedSender<RequestMessage>;
type RequestRx = UnboundedReceiver<RequestMessage>;

pub struct Interface;
impl Interface {
    pub fn get_public_socket() -> Option<SocketAddr> {
        None
    }
    pub fn get_local_ip_address() -> Option<IpAddr> {
        match local_ip_address::local_ip() {
            Ok(ip_address) => Some(ip_address),
            Err(_) => None,
        }
    }
}

struct RequestMessage {
    message: Message,
    response_channel: UnboundedSender<Message>,
}

pub enum ConnectionError {
    WebSocketHandshakeFailure,
    StreamClosedFailure,
    WebSocketReadFailure,
}

pub struct WebSocketClient;
impl WebSocketClient {
    pub async fn connect_local(port: u16) -> Result<(), ConnectionError> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        WebSocketClient::connect(address).await?;
        Ok(())
    }
    pub async fn connect(address: SocketAddr) -> Result<(), ConnectionError> {
        let uri = WebSocketClient::get_uri(address);
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
        let connection = WebSocketConnection {
            address: address,
            websocket: websocket,
            request_tx: request_tx,
        };
        let receive_task = tokio::spawn(WebSocketConnection::handle_requests(request_rx));
        let connection_task = tokio::spawn(WebSocketConnection::handle_connection(connection));

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

pub struct WebSocketServer;
impl WebSocketServer {
    pub async fn listen_local(port: u16) -> Result<(), ConnectionError> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        WebSocketServer::listen(address).await?;
        Ok(())
    }
    pub async fn listen(address: SocketAddr) -> Result<(), ConnectionError> {
        let tcp_listener = match TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(_) => return Err(ConnectionError::WebSocketHandshakeFailure),
        };

        // create transmit/receive async channels
        let (request_tx, request_rx) = mpsc::unbounded_channel();

        // create listening task
        let listen_task = tokio::spawn(WebSocketServer::listen_for_connections(
            tcp_listener,
            request_tx,
        ));

        // create receiving task
        let receive_task = tokio::spawn(WebSocketConnection::handle_requests(request_rx));

        future::select(listen_task, receive_task).await;

        Ok(())
    }

    async fn listen_for_connections(
        tcp_listener: TcpListener,
        request_tx: RequestTx,
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

                let connection = WebSocketConnection {
                    address: address,
                    websocket: websocket,
                    request_tx: request_tx_clone,
                };
                if let Err(_) = WebSocketConnection::handle_connection(connection).await {
                    println!("ws:// connection errored with: {}", address);
                    return;
                }
            });
        }
        Ok(())
    }
}

struct WebSocketConnection {
    address: SocketAddr,
    websocket: WebSocketStream<TcpStream>,
    request_tx: RequestTx,
}
impl WebSocketConnection {
    async fn handle_connection(connection: WebSocketConnection) -> Result<(), ConnectionError> {
        // split in/out channels so we can move them separately
        let (mut websocket_out, mut websocket_in) = connection.websocket.split();

        // create channel for collecting and returning responses
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        // begin response handling task
        tokio::spawn(async move {
            while let Some(response) = response_rx.recv().await {
                println!("ws:// responding to: {}", connection.address);
                match websocket_out.send(response).await {
                    Ok(()) => {}
                    Err(_) => {}
                };
            }
        });

        // read incoming websocket requests and send to message handler until the stream closes
        while let Some(message_result) = websocket_in.next().await {
            println!("ws:// request from: {}", connection.address);
            let message = match message_result {
                Ok(message) => message,
                Err(_) => return Err(ConnectionError::WebSocketReadFailure),
            };

            // send request to message handler with response channel
            if let Err(_) = connection.request_tx.send(RequestMessage {
                message: message,
                response_channel: response_tx.clone(),
            }) {
                println!("request handler channel closed for some reason");
            }
        }

        // other side closed the websocket connection
        println!("ws:// closed with: {}", connection.address);

        Ok(())
    }

    async fn handle_requests(mut request_rx: RequestRx) -> Result<(), ConnectionError> {
        while let Some(request) = request_rx.recv().await {
            let response = match request.message {
                Message::Text(text) => {
                    println!("Received text: {}", text);
                    None
                }
                Message::Binary(_) => {
                    println!("Received some binary");
                    None
                }
                Message::Ping(ping_payload) => Some(Message::Pong(ping_payload)),
                Message::Pong(_) => None,
                Message::Close(_) => None,
            };

            // if response exists, pass back response, handle error
            if let Some(response) = response {
                if let Err(err) = request.response_channel.send(response) {
                    println!(
                        "dropped response, response_rx channel was closed. {}",
                        err.to_string()
                    );
                }
            }
        }

        // transmit channels all closed gracefully
        Ok(())
    }
}
