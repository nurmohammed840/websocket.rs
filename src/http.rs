//! This module contain some utility function to work with http protocol.

use std::{collections::HashMap, fmt};

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

/// It represents an HTTP message with a schema and a header.
///
/// ### Example
///
/// ```rust
/// let bytes = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";
/// let header = web_socket::http::Http::parse(bytes).unwrap();
///
/// assert_eq!(header.prefix, "HTTP/1.1 101 Switching Protocols".to_owned());
/// assert_eq!(header.get("upgrade"), Some(&"websocket".into()));
/// assert_eq!(
///     header.get("sec-websocket-accept"),
///     Some(&"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".into())
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Http {
    /// Prefix of the http message (e.g. `HTTP/1.1 101 Switching Protocols`)
    pub prefix: String,
    ///  key-value pairs of http headers
    pub headers: HashMap<String, String>,
}

impl std::ops::Deref for Http {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.headers
    }
}

impl std::ops::DerefMut for Http {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers
    }
}

impl Http {
    fn _is_ws_upgrade(&self) -> Option<bool> {
        let upgrade = self.get("upgrade")?.eq_ignore_ascii_case("websocket");
        let connection = self.get("connection")?.eq_ignore_ascii_case("upgrade");
        Some(upgrade && connection)
    }

    /// Determine if the incoming HTTP request is an upgrade request to the WebSocket protocol.
    pub fn is_ws_upgrade(&self) -> bool {
        matches!(self._is_ws_upgrade(), Some(true))
    }

    /// get http `sec-websocket-key` header value.
    pub fn get_sec_ws_key(&self) -> Option<&str> {
        self.get("sec-websocket-version")?
            .contains("13")
            .then_some(self.get("sec-websocket-key")?)
    }

    ///
    pub fn parse(string: &str) -> Option<Self> {
        let mut lines = string.lines();
        let prefix = lines.next()?.to_owned();
        let mut headers = HashMap::default();
        for line in lines {
            if line.is_empty() {
                break;
            }
            let (key, value) = line.split_once(": ")?;
            headers.insert(key.to_ascii_lowercase(), value.into());
        }
        Some(Self { prefix, headers })
    }
}
