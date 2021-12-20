use futures::{Sink, SinkExt, Stream, StreamExt};
use std::net::SocketAddr;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub(crate) type MessageTx = UnboundedSender<(WireMessage, UnboundedSender<WireMessage>)>;
pub(crate) type MessageRx = UnboundedReceiver<(WireMessage, UnboundedSender<WireMessage>)>;

// ===== enum connection::Protocol ============================================
///
/// Defines the implemented networking protocols that are available to use.
///
#[derive(Clone, Copy)]
pub(crate) enum Protocol {
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
/// This is the raw coalescent-computer message that we send across the wire.
/// It will ultimately be wrapped by the protocol that transmits the message,
/// but at the CC abstraction layer, this is as low as it gets.
///
pub(crate) enum WireMessage {
    Empty,
    Ping(Vec<u8>),
    Pong(Vec<u8>),
}

/// ===== enum connection::Error ===============================================
///
#[derive(Debug)]
pub enum Error {
    ProtocolError,
}

/// ===== struct connection::Connection ========================================
///
pub(crate) struct Connection<TMessage, TError, TSink, TStream>
where
    TMessage: Into<WireMessage> + From<WireMessage>,
    TError: Into<Error>,
    TSink: Sink<TMessage, Error = TError>,
    TStream: Stream<Item = Result<TMessage, TError>>,
{
    pub protocol: Protocol,
    pub address: SocketAddr,
    pub sink: TSink,
    pub stream: TStream,
    pub message_tx: MessageTx,
}

/// ===== async fn handle_connection(connection: Connection) ==================
///
/// Takes ownership of a Connection struct and initiates tokio tasks to read
/// and process messages from the stream, as well as send messages to the sink.
///
pub(crate) async fn handle_connection<TMessage, TError, TSink, TStream>(
    connection: Connection<TMessage, TError, TSink, TStream>,
) -> Result<(), Error>
where
    TMessage: Into<WireMessage> + From<WireMessage> + Send,
    TError: Into<Error>,
    TSink: Sink<TMessage, Error = TError> + Unpin + Send + Sync + 'static,
    TStream: Stream<Item = Result<TMessage, TError>> + Send + Unpin,
{
    // create channel for responses to be sent through
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<WireMessage>();

    // begin a task for handling responses
    let mut sink = connection.sink; // move `sink` into a mutable owner
    tokio::spawn(async move {
        while let Some(response) = response_rx.recv().await {
            let protocol_message = response.into();
            match sink.send(protocol_message).await {
                Ok(()) => {}
                Err(_) => {}
            };
            println!(
                "{}{} response sent",
                connection.protocol, connection.address
            );
        }
    });

    // read incoming connection messages and send to message handler until the stream closes
    let mut stream = connection.stream; // move stream into mutable owner
    while let Some(protocol_message) = stream.next().await {
        println!("{}{} request recd", connection.protocol, connection.address);

        // convert protocol message into wire message
        let wire_message = match protocol_message {
            Ok(protocol_message) => protocol_message.into(),
            Err(err) => return Err(err.into()),
        };

        // send request to message handler with response channel
        if let Err(_send_error) = connection
            .message_tx
            .send((wire_message, response_tx.clone()))
        {
            println!("cannot process request, request_rx handler channel is closed");
        }
    }

    // other side closed the websocket connection
    println!("{}{} closed", connection.protocol, connection.address);

    Ok(())
}

/// ===== async fn handle_messages(message_rx: MessageRx) ==================
///
/// Takes ownership of a message receiver, and processes those messages as
/// they come in. Can spawn a new work task based on the message, and can
/// also potentially generate a response to return to the sender.
///
pub(crate) async fn handle_messages(mut message_rx: MessageRx) -> Result<(), Error> {
    while let Some(message) = message_rx.recv().await {
        let response = match message.0 {
            WireMessage::Ping(bytes) => Some(WireMessage::Pong(bytes)),
            WireMessage::Pong(_) => None,
            WireMessage::Empty => None,
        };

        // if response exists, pass back response, handle error
        if let Some(response) = response {
            if let Err(err) = message.1.send(response) {
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
