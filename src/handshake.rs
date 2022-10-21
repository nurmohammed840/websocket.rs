use std::collections::HashMap;

use sha1::{Digest, Sha1};

pub const MAGIC_STRING: &[u8; 36] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// ```rust
/// let res = [
///     "HTTP/1.1 101 Switching Protocols",
///     "Upgrade: websocket",
///     "Connection: Upgrade",
///     "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
///     "",
///     ""
/// ];
/// assert_eq!(web_socket_server::handshake::response("dGhlIHNhbXBsZSBub25jZQ==", ""), res.join("\r\n"));
/// ```
pub fn response(key: &str, headers: &str) -> String {
    let mut m = Sha1::new();
    m.update(key.as_bytes());
    m.update(MAGIC_STRING);
    let key = base64::encode(m.finalize());
    format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n{headers}\r\n",)
}

pub(crate) fn _request<'a>(
    path: &str,
    host: &str,
    headers: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> String {
    let path = path.trim_start_matches("/");
    let headers: String = headers
        .into_iter()
        .map(|(key, val)| format!("{key}: {val}\r\n"))
        .collect();

    format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n{headers}\r\n")
}

fn parse_headers(msg: &str) -> impl Iterator<Item = (&str, &str)> {
    msg.lines().filter_map(|line| line.split_once(": "))
}

pub fn get_sec_key(msg: &str) -> Option<&str> {
    let headers: HashMap<String, &str> = parse_headers(msg)
        .map(|(key, val)| (key.to_lowercase(), val))
        .collect();

    if headers.get("upgrade")?.eq_ignore_ascii_case("websocket")
        && headers.get("connection")?.eq_ignore_ascii_case("Upgrade")
        && headers.get("sec-websocket-version")?.eq(&"13")
    {
        return headers.get("sec-websocket-key").map(|&val| val);
    }
    None
}
