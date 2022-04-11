#![allow(warnings)]
use crate::CloseCode;

enum ReadyState {
    /// Socket has been created. The connection is not yet open.
    Connecting = 0,
    /// The connection is open and ready to communicate.
    Open = 1,
    /// The connection is in the process of closing.
    Closing = 2,
    /// The connection is closed or couldn't be opened.
    Closed = 3,
}

enum Event {
    /// The connection has been established, or a connection attempt has succeeded.
    Open,
    /// The connection has been closed or could not be opened.
    Close(CloseCode, String),
    /// A message has been received.
    Message(Vec<u8>),
    /// A message has been sent.
    Error(String),
}

pub struct WebSocket {
    uri: String,
    ready_state: ReadyState,

    /// the number of bytes of data that have been queued using calls to send() but not yet transmitted to the network.
    /// This value resets to zero once all queued data has been sent.
    /// This value does not reset to zero when the connection is closed; if you keep calling send(), this will continue to climb.
    buffered_amount: usize,

    pub binary_type: String,
}

impl WebSocket {
    pub fn new(url: impl Into<String>) {}

    pub fn send(&self, data: &[u8])  {
        unimplemented!()
    }

    /// Reason: The value must be no longer than 123 bytes (encoded in UTF-8)
    pub fn close(&self, code: CloseCode, reason: &str) {
        unimplemented!()
    }

    pub fn event() {

    }
}


/*
let ws = WebSocket::new("ws://echo.websocket.org");

loop {
    match ws.events().await? {
        Event::Open => {
            println!("The connection is open and ready to communicate");
        },
        Event::Message(message) => {
            println!("{}", message);
        }
    }
}
}
ws.send(b"Hello!");
*/