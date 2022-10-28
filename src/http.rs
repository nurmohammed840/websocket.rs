//! This module contain some utility function to work with http protocol.
use std::str;

/// # Example
///
/// ```rust
/// use web_socket::http::FmtHeaderField;
///
/// assert_eq!(FmtHeaderField::fmt(&("val", 2)), "val: 2\r\n");
/// assert_eq!(FmtHeaderField::fmt(&["key", "value"]), "key: value\r\n");
/// ```
pub trait FmtHeaderField {
    /// Format a single http header field
    fn fmt(_: &Self) -> String;
}

impl<T: FmtHeaderField> FmtHeaderField for &T {
    fn fmt(this: &Self) -> String {
        T::fmt(this)
    }
}
impl<T: std::fmt::Display> FmtHeaderField for [T; 2] {
    fn fmt([key, value]: &Self) -> String {
        format!("{key}: {value}\r\n")
    }
}
impl<K: std::fmt::Display, V: std::fmt::Display> FmtHeaderField for (K, V) {
    fn fmt((key, value): &Self) -> String {
        format!("{key}: {value}\r\n")
    }
}

/// ### Example
///
/// ```rust
/// let mut bytes = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n".as_bytes();
/// let header = web_socket::http::Record::from_raw(&mut bytes).unwrap();
///
/// assert_eq!(header.schema, "HTTP/1.1 101 Switching Protocols".as_bytes());
/// assert_eq!(header.get("upgrade"), Some("websocket".as_bytes()));
/// assert_eq!(
///     header.get_sec_ws_accept(),
///     Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".as_bytes())
/// );
/// ```
#[derive(Default, Clone)]
pub struct Record<'a> {
    pub schema: &'a [u8],
    pub header: Vec<(&'a [u8], &'a [u8])>,
}

const HTTP_EOF_ERR: &str = "HTTP parse error: Unexpected end";

impl<'a> Record<'a> {
    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&[u8]> {
        let key = key.as_ref();
        self.header
            .iter()
            .find_map(|(k, v)| k.eq_ignore_ascii_case(key).then_some(*v))
    }

    fn is_ws_upgrade(&self) -> Option<bool> {
        let upgrade = self.get("upgrade")?.eq_ignore_ascii_case(b"websocket");
        let connection = self.get("connection")?.eq_ignore_ascii_case(b"upgrade");
        Some(upgrade && connection)
    }

    pub fn get_sec_ws_key(&self) -> Option<&[u8]> {
        let suppoted_version = self
            .get("sec-websocket-version")?
            .windows(2)
            .any(|version| version == b"13");

        (self.is_ws_upgrade()? && suppoted_version).then_some(self.get("sec-websocket-key")?)
    }

    pub fn get_sec_ws_accept(&self) -> Option<&[u8]> {
        self.is_ws_upgrade()?
            .then_some(self.get("sec-websocket-accept")?)
    }

    pub fn from_raw(bytes: &mut &'a [u8]) -> std::result::Result<Self, &'static str> {
        let schema = trim_ascii_end(split_once(bytes, b'\n').ok_or(HTTP_EOF_ERR)?);
        let mut header = vec![];
        loop {
            match split_once(bytes, b'\n').ok_or(HTTP_EOF_ERR)? {
                b"" | b"\r" => return Ok(Self { schema, header }),
                line => {
                    let mut value = line;
                    let key = split_once(&mut value, b':')
                        .ok_or("HTTP parse error: Invalid header field")?;

                    header.push((key, trim_ascii_start(trim_ascii_end(value))));
                }
            }
        }
    }
}

impl<'a> std::fmt::Debug for Record<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut header = vec![];
        for (key, value) in &self.header {
            if let (Ok(key), Ok(value)) = (str::from_utf8(key), str::from_utf8(value)) {
                header.push((key, value))
            }
        }
        f.debug_struct("HttpRecord")
            .field("schema", &str::from_utf8(self.schema))
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
