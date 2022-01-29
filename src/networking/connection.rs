mod udp;
mod websocket;

use crate::messaging::WireMessage;

use futures::{pin_mut, Sink, SinkExt, Stream, StreamExt};
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};

pub type ConnectionUid = String;
pub type HandleSender = oneshot::Sender<ConnectionHandle>;
pub type HandleReceiver = oneshot::Receiver<ConnectionHandle>;

pub type Response = WireMessage;
pub type ResponseTx = mpsc::UnboundedSender<Response>;
pub type ResponseRx = mpsc::UnboundedReceiver<Response>;

pub type Request = (WireMessage, ResponseTx);
pub type RequestTx = mpsc::UnboundedSender<Request>;
pub type RequestRx = mpsc::UnboundedReceiver<Request>;

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
    pub uid: ConnectionUid,
    sink: Box<dyn Sink<TMessage, Error = TError> + Send + Unpin>,
    stream: Box<dyn Stream<Item = Result<TMessage, TError>> + Send + Unpin>,
}

pub struct ConnectionHandle {
    pub uid: ConnectionUid,
    outbound_tx: ResponseTx,
}
impl ConnectionHandle {
    pub fn send(&self, message: Response) -> Result<(), Error> {
        match self.outbound_tx.send(message) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ProtocolError),
        }
    }
}

/// ===== fn listen_ws(address) ===============================================
///
/// listen on a socket using websocket protocol
///
pub async fn listen_ws(address: SocketAddr) -> Result<(), Error> {
    let (connection_tx, connection_rx) = mpsc::unbounded_channel();
    tokio::spawn(websocket::listen(address, connection_tx));
    match handle_stream_connections(connection_rx).await {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::ProtocolError),
    }
}

// pub async fn listen_udp(address: SocketAddr) -> Result<(), Error> {
//     let (connection_tx, connection_rx) = mpsc::unbounded_channel();
//     tokio::spawn(udp::listen(address));
//     match handle_stream_connections(connection_rx).await {
//         Ok(_) => Ok(()),
//         Err(_) => Err(Error::ProtocolError),
//     }
// }

/// ===== fn connect_ws(address) ==============================================
///
/// connect to a socket using websocket protocol
///
pub async fn connect_ws(address: SocketAddr) -> Result<ConnectionHandle, Error> {
    // create a websocket connection
    let connection = match websocket::connect(address).await {
        Ok(conn) => conn,
        Err(_) => return Err(Error::ProtocolError),
    };

    // init the connection, and have a channel to pass back the handle
    let (handle_tx, handle_rx) = oneshot::channel();
    tokio::spawn(handle_stream_connection(connection, Some(handle_tx)));

    match handle_rx.await {
        Ok(handle) => Ok(handle),
        Err(_) => Err(Error::ProtocolError),
    }
}

/// ===== fn handle_stream_connections(connection_rx) =========================
///
/// takes a connection receiver, receives those connections until that channel
/// closes, and spawns a connection message processing task stack for each
/// connection that comes though. this should naturally close once the
/// websocket connection is closed, which should be announced by the
/// connection_rx channel being closed.
///
async fn handle_stream_connections<TMessage, TError>(
    mut connection_rx: mpsc::UnboundedReceiver<Connection<TMessage, TError>>,
) -> Result<(), Error>
where
    TMessage: Into<WireMessage> + From<WireMessage> + Send + Unpin + 'static,
    TError: Into<Error> + Send + Unpin + 'static,
{
    while let Some(connection) = connection_rx.recv().await {
        tokio::spawn(handle_stream_connection(connection, None));
    }

    // connection channel closed gracefully
    Ok(())
}

/// ===== fn handle_stream_connection(connection) =============================
///
/// takes ownership of a connection struct, and spawns tasks that handle
/// receiving messages from peers, processing messages, and sending responses
/// to peers.
///
async fn handle_stream_connection<TMessage, TError>(
    connection: Connection<TMessage, TError>,
    handle_tx: Option<HandleSender>,
) where
    TMessage: Into<WireMessage> + From<WireMessage> + Send + Unpin + 'static,
    TError: Into<Error> + Send + Unpin + 'static,
{
    let sink = connection.sink;
    let stream = connection.stream;
    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
    let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
    let recv_msg_task = tokio::spawn(handle_incoming(stream, incoming_tx, outgoing_tx.clone()));
    let proc_msg_task = tokio::spawn(process_wire_messages(incoming_rx));
    let resp_msg_task = tokio::spawn(handle_outgoing(sink, outgoing_rx));

    // get connection-handle and return
    let connection_handle = ConnectionHandle {
        uid: connection.uid,
        outbound_tx: outgoing_tx,
    };
    if let Some(tx) = handle_tx {
        if let Err(_) = tx.send(connection_handle) {
            // todo: bad send?
        }
    }

    // wait to see what happens to handlers
    match tokio::try_join!(recv_msg_task, proc_msg_task, resp_msg_task) {
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

async fn handle_incoming<TMessage, TError>(
    stream: impl Stream<Item = Result<TMessage, TError>>,
    incoming_tx: RequestTx,
    outgoing_tx: ResponseTx,
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
        if let Err(_send_error) = incoming_tx.send((wire_message, outgoing_tx.clone())) {
            println!("cannot process request, request_rx handler channel is closed");
        }
    }

    // connection stream closed gracefully
    Ok(())
}

/// ===== async fn process_wire_messages(request_rx: RequestRx) ==================
///
/// Takes ownership of a request receiver, and processes those requests as
/// they come in. Can spawn a new work task based on the request, and can
/// also potentially generate a response to return to the sender.
///
async fn process_wire_messages(mut incoming_rx: RequestRx) -> Result<(), Error> {
    while let Some(request) = incoming_rx.recv().await {
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
            WireMessage::RequestConnection => None,
            WireMessage::CloseConnection => None,
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

async fn handle_outgoing<TMessage, TError>(
    sink: impl Sink<TMessage, Error = TError>,
    outgoing_rx: ResponseRx,
) -> Result<(), Error>
where
    TMessage: Into<WireMessage> + From<WireMessage>,
    TError: Into<Error>,
{
    pin_mut!(sink, outgoing_rx);
    while let Some(response) = outgoing_rx.recv().await {
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
