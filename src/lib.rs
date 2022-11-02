#![doc = include_str!("../README.md")]

mod errors;
mod mask;
mod utils;
mod ws;

pub mod frame;
pub mod handshake;
pub mod http;
pub use ws::*;

use errors::*;
use mask::*;
use utils::*;

use std::io::Result;

use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Text,
    Binary,
}

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
    #[inline]
    pub fn data(&self) -> &[u8] {
        match self {
            Event::Ping(data) => data,
            Event::Pong(data) => data,
        }
    }
}

impl std::fmt::Display for Event<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::str::from_utf8(self.data()).unwrap_or(""))
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
