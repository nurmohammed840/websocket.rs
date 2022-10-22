use crate::http::HeaderField;
use sha1::{Digest, Sha1};

const MAGIC_STRING: &[u8; 36] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

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
/// assert_eq!(web_socket::handshake::response("dGhlIHNhbXBsZSBub25jZQ==", field), res.join("\r\n"));
/// ```
pub fn response(
    sec_key: impl AsRef<str>,
    headers: impl IntoIterator<Item = impl HeaderField>,
) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(sec_key.as_ref().as_bytes());
    sha1.update(MAGIC_STRING);
    let key = base64::encode(sha1.finalize());
    let headers: String = headers.into_iter().map(|f| HeaderField::fmt(&f)).collect();
    format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n{headers}\r\n")
}

/// ### Example
///
/// ```no_run
/// use web_socket::handshake::request;
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
    headers: impl IntoIterator<Item = impl HeaderField>,
) -> String {
    let host = host.as_ref();
    let path = path.as_ref().trim_start_matches("/");
    let sec_key = base64::encode(fastrand::u128(..).to_ne_bytes());
    let headers: String = headers.into_iter().map(|f| HeaderField::fmt(&f)).collect();
    format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {sec_key}\r\n{headers}\r\n")
}
