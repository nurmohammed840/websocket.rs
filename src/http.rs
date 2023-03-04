//! This module contain some utility function to work with http protocol.

use std::{collections::HashMap, fmt};

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

/// it represents an HTTP message with a schema and a header.
///
/// ### Example
///
/// ```rust
/// let mut bytes = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n".as_bytes();
/// let header = web_socket::http::Http::parse(&mut bytes).unwrap();
///
/// assert_eq!(header.schema, "HTTP/1.1 101 Switching Protocols".as_bytes());
/// assert_eq!(header.get("upgrade"), Some("websocket".as_bytes()));
/// assert_eq!(
///     header.get_sec_ws_accept(),
///     Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".as_bytes())
/// );
/// ```
#[derive(Default, Clone)]
pub struct Http<'a> {
    /// schema of the http message (e.g. `HTTP/1.1`)
    pub schema: &'a [u8],
    ///  key-value pairs of http headers
    pub headers: HashMap<String, &'a [u8]>,
}

const HTTP_EOF_ERR: &str = "HTTP parse error: Unexpected end";

impl<'a> Http<'a> {
    /// get http header value.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&[u8]> {
        self.headers.get(key.as_ref()).copied()
    }

    fn is_ws_upgrade(&self) -> Option<bool> {
        let upgrade = self.get("upgrade")?.eq_ignore_ascii_case(b"websocket");
        let connection = self.get("connection")?.eq_ignore_ascii_case(b"upgrade");
        Some(upgrade && connection)
    }

    /// get http `sec-websocket-key` header value.
    pub fn get_sec_ws_key(&self) -> Option<&[u8]> {
        let suppoted_version = self
            .get("sec-websocket-version")?
            .windows(2)
            .any(|version| version == b"13");

        (self.is_ws_upgrade()? && suppoted_version).then_some(self.get("sec-websocket-key")?)
    }

    /// get http `sec-websocket-accept` header value.
    pub fn get_sec_ws_accept(&self) -> Option<&[u8]> {
        self.is_ws_upgrade()?
            .then_some(self.get("sec-websocket-accept")?)
    }

    /// parse an HTTP message from a byte slice
    pub fn parse(bytes: &mut &'a [u8]) -> std::result::Result<Self, &'static str> {
        let schema = trim_ascii_end(split_once(bytes, b'\n').ok_or(HTTP_EOF_ERR)?);
        let mut header = HashMap::new();
        loop {
            match split_once(bytes, b'\n').ok_or(HTTP_EOF_ERR)? {
                b"" | b"\r" => return Ok(Self { schema, headers: header }),
                line => {
                    let mut value = line;
                    let key = split_once(&mut value, b':')
                        .ok_or("HTTP parse error: Invalid header field")?
                        .to_ascii_lowercase();

                    header.insert(
                        String::from_utf8(key).map_err(|_| "Invalid UTF-8 bytes")?,
                        trim_ascii_start(trim_ascii_end(value)),
                    );
                }
            }
        }
    }
}

impl<'a> fmt::Debug for Http<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut header = vec![];
        for (key, value) in &self.headers {
            if let Ok(value) = std::str::from_utf8(value) {
                header.push((key, value))
            }
        }
        f.debug_struct("Http")
            .field("schema", &std::str::from_utf8(self.schema))
            .field("header", &header)
            .finish()
    }
}

// --------------------------------------------------------------------------------------------------

fn split_once<'a>(reader: &mut &'a [u8], ascii: u8) -> Option<&'a [u8]> {
    let index = reader.iter().position(|&byte| ascii == byte)?;
    let val = &reader[..index];
    *reader = &reader[index + 1..];
    Some(val)
}

fn trim_ascii_start(mut bytes: &[u8]) -> &[u8] {
    while let [first, rest @ ..] = bytes {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}

fn trim_ascii_end(mut bytes: &[u8]) -> &[u8] {
    while let [rest @ .., last] = bytes {
        if last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}
