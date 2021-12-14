use futures::{future, SinkExt, StreamExt};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
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
    response_channel: UnboundedSender<Message>,
    message: Message,
}

enum ConnectionError {
    HandshakeFailure,
    StreamClosedFailure,
    WebsocketReadFailure,
}

pub struct WebsocketIPCServer {}

impl WebsocketIPCServer {
    pub fn new() -> WebsocketIPCServer {
        WebsocketIPCServer {}
    }

    pub async fn start(&self) {
        // create transmit/receive async channels
        let (request_tx, request_rx) = mpsc::unbounded_channel();

        // create listening task
        let listen_task = tokio::spawn(WebsocketIPCServer::listen_for_ipc_connections(
            "127.0.0.1:5000",
            request_tx,
        ));

        // create receiving task
        let receive_task = tokio::spawn(WebsocketIPCServer::receive_messages(request_rx));

        future::select(listen_task, receive_task).await;
    }

    async fn listen_for_ipc_connections<T>(
        address: T,
        request_tx: RequestTx,
    ) -> Result<(), std::io::Error>
    where
        T: ToSocketAddrs,
    {
        // get listener from address
        let listener = match TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(error) => return Err(error),
        };

        // accept connections until socket is closed
        while let Ok((stream, address)) = listener.accept().await {
            tokio::spawn(WebsocketIPCServer::handle_local_connection(
                address,
                stream,
                request_tx.clone(),
            ));
        }

        // return ok
        Ok(())
    }

    async fn handle_local_connection(
        address: SocketAddr,
        tcp_stream: TcpStream,
        request_tx: RequestTx,
    ) -> Result<(), ConnectionError> {
        println!("Incoming local TCP connection from: {}", address);

        // accept TCP stream as websocket connection
        let websocket = match tokio_tungstenite::accept_async(tcp_stream).await {
            Ok(socket) => socket,
            Err(_) => return Err(ConnectionError::HandshakeFailure),
        };
        println!("WebSocket connection established with: {}", address);

        // split the websocket
        let (mut outgoing, mut incoming) = websocket.split();

        // create a response channel for processed messages
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        // receive responses from message handler and send to websocket caller
        tokio::spawn(async move {
            while let Some(response) = response_rx.recv().await {
                println!("Sending response to: {}", address);
                match outgoing.send(response).await {
                    Ok(()) => {}
                    Err(_) => {}
                };
            }
        });

        // read incoming websocket requests and send to message handler until the stream closes
        while let Some(message_result) = incoming.next().await {
            // unwrap message
            let message = match message_result {
                Ok(message) => message,
                Err(_) => return Err(ConnectionError::WebsocketReadFailure),
            };
            println!("Received request from: {}", address);

            // send request to message handler with response channel
            if let Err(_) = request_tx.send(RequestMessage {
                response_channel: response_tx.clone(),
                message: message,
            }) {
                // message handler receiver closed
            }
        }

        println!("WebSocket connection shutting down with: {}", address);

        Ok(())
    }

    async fn receive_messages(mut request_rx: RequestRx) {
        while let Some(request) = request_rx.recv().await {
            // match request_message.message {
            //     Message::Text(_) => todo!(),
            //     Message::Binary(_) => todo!(),
            //     Message::Ping(_) => todo!(),
            //     Message::Pong(_) => todo!(),
            //     Message::Close(_) => todo!(),
            // }

            if let Err(_) = request.response_channel.send(Message::Pong(vec![0u8; 32])) {
                // couldn't send the response, channel is closed
            }
        }
    }
}
