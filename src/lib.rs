#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod errors;
mod mask;
mod message;
mod utils;
mod ws;

pub mod handshake;
pub mod http;
pub use ws::*;

use mask::*;
use utils::*;

use std::io::Result;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
};

/// Used to represent `WebSocket<SERVER>` type.
pub const SERVER: bool = true;
/// Used to represent `WebSocket<CLIENT>` type.
pub const CLIENT: bool = false;

/// This trait is responsible for encoding websocket messages.
pub trait Message {
    /// Encode websocket data frame.
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>);
}

/// This trait is responsible for encoding websocket closed frame.
pub trait CloseFrame {
    /// Encoded websocked close frame
    type Bytes;
    /// Encode websocket close frame.
    fn encode<const SIDE: bool>(self) -> Self::Bytes;
}

/// The [CloseEvent] enum represents the possible events that can occur when a WebSocket connection is closed.
/// It has two variants: `Error` and `Close`.
#[derive(Debug)]
pub enum CloseEvent {
    /// represents the websocket error message.
    Error(&'static str),
    /// represents a successful close event of the WebSocket connection.
    Close {
        /// represents the status [CloseCode] of the close event.
        code: u16,
        /// represents the reason for the close event
        reason: Box<str>,
    },
}

/// It represent the type of data that is being sent over the WebSocket connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    /// `Text` data is represented as a sequence of Unicode characters encoded using UTF-8 encoding.
    Text,
    /// `Binary` data can be any sequence of bytes and is typically used for sending non-textual data, such as images, audio files etc...
    Binary,
}

/// Represent a websocket event, either `Ping` or `Pong`
#[derive(Debug, Clone)]
pub enum Event<'a> {
    /// A Ping frame may serve either as a keepalive or as a means to verify that the remote endpoint is still responsive.
    Ping(&'a [u8]),

    /// A Pong frame sent in response to a Ping frame must have identical
    /// "Application data" as found in the message body of the Ping frame being replied to.
    ///
    ///  A Pong frame MAY be sent unsolicited.  This serves as a unidirectional heartbeat.  A response to an unsolicited Pong frame is not expected.
    Pong(&'a [u8]),
}

impl Event<'_> {
    /// Returns the slice of data contained within a `Ping` or `Pong` frame.
    #[inline]
    pub fn data(&self) -> &[u8] {
        match self {
            Event::Ping(data) => data,
            Event::Pong(data) => data,
        }
    }
}

/// When closing an established connection an endpoint MAY indicate a reason for closure.
#[derive(Debug, Clone, Copy)]
pub enum CloseCode {
    /// The purpose for which the connection was established has been fulfilled
    Normal = 1000,
    /// Server going down or a browser having navigated away from a page
    Away = 1001,
    /// An endpoint is terminating the connection due to a protocol error.
    ProtocolError = 1002,
    /// It has received a type of data it cannot accept
    Unsupported = 1003,

    // reserved 1004
    /// MUST NOT be set as a status code in a Close control frame by an endpoint.
    ///
    /// No status code was actually present.
    NoStatusRcvd = 1005,
    /// MUST NOT be set as a status code in a Close control frame by an endpoint.
    ///
    /// Connection was closed abnormally.
    Abnormal = 1006,
    /// Application has received data within a message that was not consistent with the type of the message.
    InvalidPayload = 1007,
    /// This is a generic status code that can be returned when there is no other more suitable status code.
    PolicyViolation = 1008,
    /// Message that is too big for it to process.
    MessageTooBig = 1009,
    /// It has expected the server to negotiate one or more extension.
    MandatoryExt = 1010,
    /// The server has encountered an unexpected condition that prevented it from fulfilling the request.
    InternalError = 1011,
    /// MUST NOT be set as a status code in a Close control frame by an endpoint.
    ///
    /// The connection was closed due to a failure to perform a TLS handshake.
    TLSHandshake = 1015,
}

impl From<CloseCode> for u16 {
    #[inline]
    fn from(code: CloseCode) -> Self {
        code as u16
    }
}

impl From<u16> for CloseCode {
    #[inline]
    fn from(value: u16) -> Self {
        match value {
            1000 => CloseCode::Normal,
            1001 => CloseCode::Away,
            1002 => CloseCode::ProtocolError,
            1003 => CloseCode::Unsupported,
            1005 => CloseCode::NoStatusRcvd,
            1006 => CloseCode::Abnormal,
            1007 => CloseCode::InvalidPayload,
            1009 => CloseCode::MessageTooBig,
            1010 => CloseCode::MandatoryExt,
            1011 => CloseCode::InternalError,
            1015 => CloseCode::TLSHandshake,
            _ => CloseCode::PolicyViolation,
        }
    }
}
