use sha1::{Digest, Sha1};

pub const MAGIC_STRING: &[u8; 36] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// ### Example
///
/// ```rust
/// use web_socket::handshake::sec_accept_key_from;
/// assert_eq!(sec_accept_key_from("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
/// ```
pub fn sec_accept_key_from(sec_key: impl AsRef<str>) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(sec_key.as_ref().as_bytes());
    sha1.update(MAGIC_STRING);
    base64::encode(sha1.finalize())
}

pub fn sec_key() -> String {
    base64::encode(fastrand::u128(..).to_ne_bytes())
}

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
/// assert_eq!(web_socket::handshake::response!("dGhlIHNhbXBsZSBub25jZQ=="), res.join("\r\n"));
/// ```
#[macro_export]
macro_rules! response {
    [$sec_key: expr] => ({
        web_socket::handshake::response!($sec_key, [] as [(&str, &str); 0])
    });
    [$sec_key: expr, $headers: expr] => ({
        let key = web_socket::handshake::sec_accept_key_from($sec_key);
        let headers: String = $headers.iter().map(|(key, val)| format!("{key}: {val}\r\n")).collect();
        format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n{headers}\r\n")
    });
}

/// ### Example
///
/// ```rust
/// let _ = web_socket::handshake::request!("example.com", "/path");
/// ```
#[macro_export]
macro_rules! request {
    [$host: expr] => ({
        web_socket::handshake::request!($host, "/", [] as [(&str, &str); 0])
    });
    [$host: expr, $path: expr] => ({
        web_socket::handshake::request!($host, $path, [] as [(&str, &str); 0])
    });
    [$host: expr, $path: expr, $headers: expr] => ({
        let host = &$host;
        let path = $path.trim_start_matches("/");
        let sec_key = web_socket::handshake::sec_key();
        let headers: String = $headers.iter().map(|(key, val)| format!("{key}: {val}\r\n")).collect();
        format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {sec_key}\r\n{headers}\r\n")
    });
}

pub use request;
pub use response;
