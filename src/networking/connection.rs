mod websocket;

use futures::{pin_mut, Sink, SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub type Response = WireMessage;
pub type ResponseTx = UnboundedSender<Response>;
pub type ResponseRx = UnboundedReceiver<Response>;

pub type Request = (WireMessage, ResponseTx);
pub type RequestTx = UnboundedSender<Request>;
pub type RequestRx = UnboundedReceiver<Request>;

// ===== enum connection::Protocol ============================================
///
/// Defines the implemented networking protocols that are available to use.
///
#[derive(Clone, Copy)]
pub enum Protocol {
    WebSocket,
}
impl core::fmt::Display for Protocol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let protocol_string = match self {
            Protocol::WebSocket => "ws://",
        };
        write!(f, "{}", protocol_string)
    }
}

// ===== enum connection::WireMessage =========================================
///
/// This is the raw coalescent-swarm message that we send across the wire.
/// It will ultimately be wrapped by the protocol that transmits the message,
/// but at the c-swarm abstraction layer, this is as low as it gets.
///
#[derive(Serialize, Deserialize)]
pub enum WireMessage {
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    ApiCall { function: String, data: Vec<u8> },
    Empty,
}

/// ===== enum connection::Error ==============================================
///
#[derive(Debug)]
pub enum Error {
    ProtocolError,
}

/// ===== struct connection::Connection =======================================
///
pub struct Connection<TMessage, TError>
where
    TMessage: Into<WireMessage> + From<WireMessage>,
    TError: Into<Error>,
{
    sink: Box<dyn Sink<TMessage, Error = TError> + Send + Unpin>,
    stream: Box<dyn Stream<Item = Result<TMessage, TError>> + Send + Unpin>,
}

/// ===== fn listen_ws(address) ===============================================
///
/// listen on a socket using websocket protocol
///
pub async fn listen_ws(address: SocketAddr) -> Result<(), Error> {
    let (connection_tx, connection_rx) = mpsc::unbounded_channel();
    tokio::spawn(websocket::listen(address, connection_tx));
    match handle_connections(connection_rx).await {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::ProtocolError),
    }
}

/// ===== fn connect_ws(address) ==============================================
///
/// connect to a socket using websocket protocol
///
pub async fn connect_ws(address: SocketAddr) -> Result<(), Error> {
    let connection = match websocket::connect(address).await {
        Ok(conn) => conn,
        Err(_) => return Err(Error::ProtocolError),
    };
    handle_connection(connection).await;

    Ok(())
}

/// ===== fn handle_connections(connection_rx) ================================
///
/// takes a connection receiver, receives those connections until that channel
/// closes, and spawns a connection message processing task stack for each
/// connection that comes though. this should naturally close once the
/// websocket connection is closed, which should be announced by the
/// connection_rx channel being closed.
///
async fn handle_connections<TMessage, TError>(
    mut connection_rx: UnboundedReceiver<Connection<TMessage, TError>>,
) -> Result<(), Error>
where
    TMessage: Into<WireMessage> + From<WireMessage> + Send + Unpin + 'static,
    TError: Into<Error> + Send + Unpin + 'static,
{
    while let Some(connection) = connection_rx.recv().await {
        tokio::spawn(handle_connection(connection));
    }

    // connection channel closed gracefully
    Ok(())
}

/// ===== fn handle_connection(connection) ====================================
///
/// takes ownership of a connection struct, and spawns tasks that handle
/// receiving messages from peers, processing messages, and sending responses
/// to peers.
///
async fn handle_connection<TMessage, TError>(connection: Connection<TMessage, TError>)
where
    TMessage: Into<WireMessage> + From<WireMessage> + Send + Unpin + 'static,
    TError: Into<Error> + Send + Unpin + 'static,
{
    let sink = connection.sink;
    let stream = connection.stream;
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let (response_tx, response_rx) = mpsc::unbounded_channel();
    let receive_messages = tokio::spawn(handle_raw_messages(stream, request_tx, response_tx));
    let process_messages = tokio::spawn(handle_wire_messages(request_rx));
    let send_messages = tokio::spawn(handle_responses(sink, response_rx));

    // wait to see what happens to handlers
    match tokio::try_join!(receive_messages, process_messages, send_messages) {
        Ok((receive, process, send)) => {
            if let Err(_) = receive {
                // receive error
            }
            if let Err(_) = process {
                // process error
            }
            if let Err(_) = send {
                // send error
            }
        }
        Err(_) => {
            // join error
        }
    };
}

async fn handle_raw_messages<TMessage, TError>(
    stream: impl Stream<Item = Result<TMessage, TError>>,
    request_tx: RequestTx,
    response_tx: ResponseTx,
) -> Result<(), Error>
where
    TMessage: Into<WireMessage> + From<WireMessage>,
    TError: Into<Error>,
{
    pin_mut!(stream);
    while let Some(protocol_message) = stream.next().await {
        let wire_message = match protocol_message {
            Ok(protocol_message) => protocol_message.into(),
            Err(err) => return Err(err.into()),
        };
        if let Err(_send_error) = request_tx.send((wire_message, response_tx.clone())) {
            println!("cannot process request, request_rx handler channel is closed");
        }
    }

    // connection stream closed gracefully
    Ok(())
}

/// ===== async fn handle_wire_messages(request_rx: RequestRx) ==================
///
/// Takes ownership of a request receiver, and processes those requests as
/// they come in. Can spawn a new work task based on the request, and can
/// also potentially generate a response to return to the sender.
///
async fn handle_wire_messages(mut request_rx: RequestRx) -> Result<(), Error> {
    while let Some(request) = request_rx.recv().await {
        let (wire_message, response_tx) = request;

        if let Some(response) = match wire_message {
            WireMessage::ApiCall { function, data } => {
                println!(
                    "WireMessage::ApiCall(fn: {}, input_bytes: {})",
                    function,
                    data.len()
                );
                None
            }
            WireMessage::Ping(bytes) => Some(WireMessage::Pong(bytes)),
            WireMessage::Pong(_) => None,
            WireMessage::Empty => None,
        } {
            if let Err(err) = response_tx.send(response) {
                println!(
                    "dropped response, response_rx channel was closed. {}",
                    err.to_string()
                );
            }
        }
    }

    // request channel closed gracefully
    Ok(())
}

async fn handle_responses<TMessage, TError>(
    sink: impl Sink<TMessage, Error = TError>,
    response_rx: ResponseRx,
) -> Result<(), Error>
where
    TMessage: Into<WireMessage> + From<WireMessage>,
    TError: Into<Error>,
{
    pin_mut!(sink, response_rx);
    while let Some(response) = response_rx.recv().await {
        match sink.send(response.into()).await {
            Ok(()) => {}
            Err(_) => {
                // sink err?
            }
        };
    }

    // response channel closed gracefully
    Ok(())
}
