use futures::{future, SinkExt, StreamExt};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::tungstenite::Message;

type RequestTx = UnboundedSender<RequestMessage>;
type RequestRx = UnboundedReceiver<RequestMessage>;

pub struct Interface;

impl Interface {
    pub fn get_public_socket() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000)
    }
}

struct RequestMessage {
    message: Message,
    response_channel: UnboundedSender<Message>,
}

enum ConnectionError {
    WebSocketHandshakeFailure,
    StreamClosedFailure,
    WebSocketReadFailure,
}

pub struct WebSocketServer {
    address: SocketAddr,
}

impl WebSocketServer {
    pub fn new(address: SocketAddr) -> WebSocketServer {
        WebSocketServer { address }
    }
    pub fn new_local(port: u16) -> WebSocketServer {
        WebSocketServer {
            address: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    pub async fn start(&self) {
        let tcp_listener = match TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(_) => return,
        };

        // create transmit/receive async channels
        let (request_tx, request_rx) = mpsc::unbounded_channel();

        // create listening task
        let listen_task = tokio::spawn(WebSocketServer::listen_for_connections(
            tcp_listener,
            request_tx,
        ));

        // create receiving task
        let receive_task = tokio::spawn(WebSocketServer::handle_requests(request_rx));

        future::select(listen_task, receive_task).await;
    }

    async fn listen_for_connections(
        tcp_listener: TcpListener,
        request_tx: RequestTx,
    ) -> Result<(), std::io::Error> {
        while let Ok((stream, address)) = tcp_listener.accept().await {
            tokio::spawn(WebSocketServer::handle_connection(
                address,
                stream,
                request_tx.clone(),
            ));
        }
        Ok(())
    }

    async fn handle_connection(
        address: SocketAddr,
        tcp_stream: TcpStream,
        request_tx: RequestTx,
    ) -> Result<(), ConnectionError> {
        println!("Incoming TCP connection from: {}", address);

        let websocket = match tokio_tungstenite::accept_async(tcp_stream).await {
            Ok(socket) => socket,
            Err(_) => return Err(ConnectionError::WebSocketHandshakeFailure),
        };
        println!("ws:// opened with: {}", address);

        // split in/out channels so we can move them separately
        let (mut websocket_out, mut websocket_in) = websocket.split();
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        //
        tokio::spawn(async move {
            while let Some(response) = response_rx.recv().await {
                println!("ws:// responding to: {}", address);
                match websocket_out.send(response).await {
                    Ok(()) => {}
                    Err(_) => {}
                };
            }
        });

        // read incoming websocket requests and send to message handler until the stream closes
        while let Some(message_result) = websocket_in.next().await {
            println!("ws:// request from: {}", address);
            let message = match message_result {
                Ok(message) => message,
                Err(_) => return Err(ConnectionError::WebSocketReadFailure),
            };

            // send request to message handler with response channel
            if let Err(_) = request_tx.send(RequestMessage {
                message: message,
                response_channel: response_tx.clone(),
            }) {
                println!("request handler");
            }
        }
        // other

        println!("ws:// closed with: {}", address);

        Ok(())
    }

    async fn handle_requests(mut request_rx: RequestRx) -> Result<(), ConnectionError> {
        while let Some(request) = request_rx.recv().await {
            // process request
            // match request_message.message {
            //     Message::Text(_) => todo!(),
            //     Message::Binary(_) => todo!(),
            //     Message::Ping(_) => todo!(),
            //     Message::Pong(_) => todo!(),
            //     Message::Close(_) => todo!(),
            // }
            let response = Message::Pong(vec![0u8; 32]);

            // pass back response, handle error
            if let Err(err) = request.response_channel.send(response) {
                println!(
                    "dropped response, response_rx channel was closed. {}",
                    err.to_string()
                );
            }
        }

        // transmit channels all closed gracefully
        Ok(())
    }
}
