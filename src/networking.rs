pub mod websocket;

use futures::{Sink, SinkExt, Stream, StreamExt};
use std::net::{IpAddr, SocketAddr};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

type RequestTx = UnboundedSender<Request>;
type RequestRx = UnboundedReceiver<Request>;

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

enum Protocol {
    WebSocket,
}

enum Message {
    Empty,
}

struct Response;

impl Into<Message> for Response {
    fn into(self) -> Message {
        Message::Empty
    }
}
struct Request {
    message: Message,
    response_channel: UnboundedSender<Response>,
}

pub enum ConnectionError {
    WebSocketHandshakeFailure,
    StreamClosedFailure,
    WebSocketReadFailure,
}

struct Connection<TMessage, TError, TSink, TStream>
where
    TMessage: Into<Message> + From<Response>,
    TError: Into<ConnectionError>,
    TSink: Sink<TMessage, Error = TError>, // + Send + Unpin + 'static,
    TStream: Stream<Item = Result<TMessage, TError>>, // + Unpin,
{
    protocol: Protocol,
    address: SocketAddr,
    stream: TStream,
    sink: TSink,
    request_tx: RequestTx,
}

impl<TMessage, TError, TSink, TStream> Connection<TMessage, TError, TSink, TStream>
where
    TMessage: Into<Message> + From<Response> + Send,
    TError: Into<ConnectionError>,
    TSink: Sink<TMessage, Error = TError> + Unpin + Send + Sync + 'static,
    TStream: Stream<Item = Result<TMessage, TError>> + Send + Unpin,
{
    async fn handle_connection<'a>(
        // &'static mut self,
        connection: Connection<TMessage, TError, TSink, TStream>,
    ) -> Result<(), ConnectionError> {
        // create channel for collecting and returning responses
        let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Response>();

        // begin response handling task
        let mut sink = connection.sink; // move into mutable owner
        tokio::spawn(async move {
            while let Some(response) = response_rx.recv().await {
                println!("ws:// responding to: {}", connection.address);
                match sink.send(response.into()).await {
                    Ok(()) => {}
                    Err(_) => {}
                };
            }
        });

        // read incoming websocket requests and send to message handler until the stream closes
        let mut stream = connection.stream; // move into mutable owner
        while let Some(stream_message) = stream.next().await {
            println!("ws:// request from: {}", connection.address);

            // convert stream message into wire message
            let wire_message = match stream_message {
                Ok(message) => message.into(),
                Err(err) => return Err(err.into()),
            };

            // send request to message handler with response channel
            if let Err(_) = connection.request_tx.send(Request {
                message: wire_message,
                response_channel: response_tx.clone(),
            }) {
                println!("request handler channel closed for some reason");
            }
        }

        // other side closed the websocket connection
        println!("ws:// closed with: {}", connection.address);

        Ok(())
    }
}

async fn handle_requests(mut request_rx: RequestRx) -> Result<(), ConnectionError> {
    while let Some(request) = request_rx.recv().await {
        let response = match request.message {
            Message::Empty => Some(Response {}),
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
