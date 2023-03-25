//! # Client handshake request
//!
//! A client sends a handshake request to the server. It includes the following information:
//!
//! ```yml
//! GET /chat HTTP/1.1
//! Host: example.com:8000
//! Upgrade: websocket
//! Connection: Upgrade
//! Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
//! Sec-WebSocket-Version: 13
//! ```
//!
//! The server must be careful to understand everything the client asks for, otherwise security issues can occur.
//! If any header is not understood or has an incorrect value, the server should send a 400 ("Bad Request")} response and immediately close the socket.
//!
//! ### Tips
//!
//! All browsers send an Origin header.
//! You can use this header for security (checking for same origin, automatically allowing or denying, etc.) and send a 403 Forbidden if you don't like what you see.
//! However, be warned that non-browser agents can send a faked Origin. Most applications reject requests without this header.
//!
//! Any http headers is allowed. (Do whatever you want with them)
//!
//! ### Note
//!
//! -  HTTP version must be `1.1` or greater, and method must be `GET`
//! - `Host` header field containing the server's authority.
//! - `Upgrade` header field containing the value `"websocket"`
//! - `Connection` header field that includes the token `"Upgrade"`
//! - `Sec-WebSocket-Version` header field containing the value `13`
//! - `Sec-WebSocket-Key` header field with a base64-encoded value that, when decoded, is 16 bytes in length.
//! -  Request may include any other header fields, for example, cookies and/or authentication-related header fields.
//! -  Optionally, `Origin` header field.  This header field is sent by all browser clients.

use sha1::{Digest, Sha1};
use std::fmt;

/// WebSocket magic string used during the WebSocket handshake
pub const MAGIC_STRING: &[u8; 36] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Create `Sec-WebSocket-Accept` key from `Sec-WebSocket-Key` http header value.
///
/// ### Example
///
/// ```rust
/// use crate::utils::handshake::accept_key_from;
/// assert_eq!(accept_key_from("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
/// ```
#[inline]
pub fn accept_key_from(sec_ws_key: impl AsRef<[u8]>) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(sec_ws_key.as_ref());
    sha1.update(MAGIC_STRING);
    base64_encode(sha1.finalize())
}

/// ## Server handshake response
///
/// When the server receives the handshake request,
/// It should send back a special response that indicates that the protocol will be changing from HTTP to WebSocket.
///
/// The `Sec-WebSocket-Accept` header is important in that the server must derive it from the `Sec-WebSocket-Key` that the client sent to it.
///
/// ### Example
///
/// ```rust
/// let res = [
///     "HTTP/1.1 101 Switching Protocols",
///     "Upgrade: websocket",
///     "Connection: Upgrade",
///     "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
///     "",
///     ""
/// ];
/// let field: Option<(&str, &str)> = None;
/// assert_eq!(crate::utils::handshake::response("dGhlIHNhbXBsZSBub25jZQ==", field), res.join("\r\n"));
/// ```
///
/// To get it, concatenate the client's `Sec-WebSocket-Key` and the string _"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"_ together (it's a [Magic string](https://en.wikipedia.org/wiki/Magic_string)), take the SHA-1 hash of the result, and return the base64 encoding of that hash.
///
///
/// 1. If the connection is happening on an HTTPS (HTTP-over-TLS) port,
///    perform a TLS handshake over the connection.  If this fails
///    (e.g., the client indicated a host name in the extended client
///    hello "server_name" extension that the server does not host),
///    then close the connection.
///
/// 2. The server can perform additional client authentication, Or The server MAY redirect the client.
///
///
/// ### Note
///
/// - Regular HTTP status codes can be used only before the handshake. After the handshake succeeds, you have to use a different set of codes (defined in section 7.4 of the spec)
pub fn response(
    sec_ws_key: impl AsRef<[u8]>,
    headers: impl IntoIterator<Item = impl Header>,
) -> String {
    let key = accept_key_from(sec_ws_key);
    let headers: String = headers.into_iter().map(|f| Header::fmt(&f)).collect();
    format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n{headers}\r\n")
}

/// Create websocket handshake request
///
/// ### Example
///
/// ```no_run
/// use crate::utils::handshake::request;
/// let _ = request("example.com", "/path", [("key", "value")]);
/// ```
///
/// ### Output
///
/// ```yaml
/// GET /path HTTP/1.1
/// Host: example.com
/// Upgrade: websocket
/// Connection: Upgrade
/// Sec-WebSocket-Version: 13
/// Sec-WebSocket-Key: D3E1sFZlZfeZgNXtVHfhKg== # randomly generated
/// key: value
/// ...
/// ```
pub fn request(
    host: impl AsRef<str>,
    path: impl AsRef<str>,
    headers: impl IntoIterator<Item = impl Header>,
) -> (String, String) {
    let host = host.as_ref();
    let path = path.as_ref().trim_start_matches('/');
    let sec_key = base64_encode(42_u128.to_ne_bytes());
    let headers: String = headers.into_iter().map(|f| Header::fmt(&f)).collect();
    (format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {sec_key}\r\n{headers}\r\n"), sec_key)
}

/// Provides a interface for formatting HTTP headers
///
/// # Example
///
/// ```rust
/// use web_socket::http::Header;
///
/// assert_eq!(Header::fmt(&("val", 2)), "val: 2\r\n");
/// assert_eq!(Header::fmt(&["key", "value"]), "key: value\r\n");
/// ```
pub trait Header {
    /// Format a single http header field
    fn fmt(_: &Self) -> String;
}

impl<T: Header> Header for &T {
    fn fmt(this: &Self) -> String {
        T::fmt(this)
    }
}
impl<T: fmt::Display> Header for [T; 2] {
    fn fmt([key, value]: &Self) -> String {
        format!("{key}: {value}\r\n")
    }
}
impl<K: fmt::Display, V: fmt::Display> Header for (K, V) {
    fn fmt((key, value): &Self) -> String {
        format!("{key}: {value}\r\n")
    }
}

fn base64_encode(string: impl AsRef<[u8]>) -> String {
    base64::Engine::encode(&base64::prelude::BASE64_STANDARD, string)
}
