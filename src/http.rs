/// This module contain some utility function to work with http protocol.

use std::collections::HashMap;

/// # Example
///
/// ```rust
/// use web_socket::http::HeaderField;
///
/// assert_eq!(HeaderField::fmt(&("val", 2)), "val: 2\r\n");
/// assert_eq!(HeaderField::fmt(&["key", "value"]), "key: value\r\n");
/// ```
pub trait HeaderField {
    /// Format a single http header field
    fn fmt(_: &Self) -> String;
}

impl<T: HeaderField> HeaderField for &T {
    fn fmt(this: &Self) -> String {
        T::fmt(this)
    }
}
impl<T: std::fmt::Display> HeaderField for [T; 2] {
    fn fmt([key, value]: &Self) -> String {
        format!("{key}: {value}\r\n")
    }
}
impl<K: std::fmt::Display, V: std::fmt::Display> HeaderField for (K, V) {
    fn fmt((key, value): &Self) -> String {
        format!("{key}: {value}\r\n")
    }
}

/// ### Example
///
/// ```rust
/// let req = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";
/// let headers = web_socket::http::headers_from_raw(req);
///
/// assert_eq!(headers.get("upgrade"), Some(&"websocket"));
/// assert_eq!(headers.get("connection"), Some(&"Upgrade"));
/// assert_eq!(headers.get("sec-websocket-accept"), Some(&"s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
/// ```
pub fn headers_from_raw(str: &str) -> HashMap<String, &str> {
    str.lines()
        .filter_map(|line| line.split_once(": "))
        .map(|(key, val)| (key.to_lowercase(), val))
        .collect()
}

/// ### Example
///
/// ```rust
/// use web_socket::http::SecWebSocketKey;
///
/// let req = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";
/// let headers = web_socket::http::headers_from_raw(req);
///
/// assert_eq!(headers.get_sec_ws_accept_key(), Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
/// ```
pub trait SecWebSocketKey {
    fn get_sec_ws_key(&self) -> Option<&str>;
    fn get_sec_ws_accept_key(&self) -> Option<&str>;
}

impl SecWebSocketKey for HashMap<String, &str> {
    fn get_sec_ws_key(&self) -> Option<&str> {
        if is_ws_upgrade_req(&self)? && self.get("sec-websocket-version")?.contains("13") {
            return self.get("sec-websocket-key").cloned();
        }
        None
    }

    fn get_sec_ws_accept_key(&self) -> Option<&str> {
        if is_ws_upgrade_req(&self)? {
            return self.get("sec-websocket-accept").cloned();
        }
        None
    }
}

fn is_ws_upgrade_req(headers: &HashMap<String, &str>) -> Option<bool> {
    let is_upgrade = headers.get("upgrade")?.eq_ignore_ascii_case("websocket")
        && headers.get("connection")?.eq_ignore_ascii_case("upgrade");

    Some(is_upgrade)
}
