use sha1::{Digest, Sha1};
use std::collections::HashMap;

const _EMPTY_HEADER: [(&str, &str); 0] = [];
pub const MAGIC_STRING: &[u8; 36] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub fn sec_ws_accept_key_from(sec_ws_key: impl AsRef<str>) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(sec_ws_key.as_ref().as_bytes());
    sha1.update(MAGIC_STRING);
    base64::encode(sha1.finalize())
}

pub fn sec_ws_key() -> String {
    base64::encode(fastrand::u128(..).to_ne_bytes())
}

/// ```rust
/// let res = [
///     "HTTP/1.1 101 Switching Protocols",
///     "Upgrade: websocket",
///     "Connection: Upgrade",
///     "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
///     "",
///     ""
/// ];
/// assert_eq!(web_socket_server::handshake::response!("dGhlIHNhbXBsZSBub25jZQ=="), res.join("\r\n"));
/// ```
#[macro_export]
macro_rules! response {
    [$sec_ws_key: expr] => { response!($sec_ws_key, _EMPTY_HEADER) };
    [$sec_ws_key: expr, $headers: expr] => ({
        let key = sec_ws_accept_key_from($sec_ws_key);
        let headers: String = $headers.iter().map(|(key, val)| format!("{key}: {val}\r\n")).collect();
        format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n{headers}\r\n")
    });
}

#[macro_export]
macro_rules! request {
    [$host: expr] => ({
        request!($host, "/", _EMPTY_HEADER)
    });
    [$host: expr, $path: expr] => ({
        request!($host, $path, _EMPTY_HEADER)
    });
    [$host: expr, $path: expr, $headers: expr] => ({
        let host = &$host;
        let path = $path.trim_start_matches("/");
        let sec_key = sec_ws_key();
        let headers: String = $headers.iter().map(|(key, val)| format!("{key}: {val}\r\n")).collect();
        format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {sec_key}\r\n{headers}\r\n")
    });
}


pub fn headers_from_raw(str: &str) -> HashMap<String, &str> {
    str.lines()
        .filter_map(|line| line.split_once(": "))
        .map(|(key, val)| (key.to_lowercase(), val))
        .collect()
}

pub trait GetSecKey {
    fn get_sec_ws_key(&self) -> Option<&str>;
    fn get_sec_ws_accept_key(&self) -> Option<&str>;
}

impl GetSecKey for HashMap<String, &str> {
    fn get_sec_ws_key(&self) -> Option<&str> {
        if is_ws_upgrade(&self)? && self.get("sec-websocket-version")?.contains("13") {
            return self.get("sec-websocket-key").cloned();
        }
        None
    }

    fn get_sec_ws_accept_key(&self) -> Option<&str> {
        if is_ws_upgrade(&self)? {
            return self.get("sec-websocket-accept").cloned();
        }
        None
    }
}

fn is_ws_upgrade(headers: &HashMap<String, &str>) -> Option<bool> {
    let is_upgrade = headers.get("upgrade")?.eq_ignore_ascii_case("websocket")
        && headers.get("connection")?.eq_ignore_ascii_case("upgrade");

    Some(is_upgrade)
}