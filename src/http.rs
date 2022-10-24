//! This module contain some utility function to work with http protocol.
use std::ops::Deref;

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

impl HeaderField for httparse::Header<'_> {
    fn fmt(Self { name, value }: &Self) -> String {
        format!("{name}: {}\r\n", std::str::from_utf8(value).unwrap_or(""))
    }
}

/// ### Example
///
/// ```rust
/// let mut bytes = "Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n".as_bytes();
/// let mut header = Header::default();
/// header.parse_from_raw(&mut bytes).unwrap();
///
/// assert_eq!(header.get("upgrade"), Some("websocket".as_bytes()));
/// assert_eq!(header.get_as_str("connection"), Some("Upgrade"));
/// assert_eq!(header.get_sec_ws_accept_key(), Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".as_bytes()));
/// ```
pub struct Header<'a, const N: usize = 16> {
    init: usize,
    inner: [httparse::Header<'a>; N],
}

impl Default for Header<'_> {
    fn default() -> Self {
        Self {
            init: 0,
            inner: [httparse::EMPTY_HEADER; 16],
        }
    }
}

impl<'a, const N: usize> Header<'a, N> {
    pub const fn new() -> Self {
        Self {
            init: 0,
            inner: [httparse::EMPTY_HEADER; N],
        }
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<&[u8]> {
        self.iter()
            .find(|header| header.name.eq_ignore_ascii_case(key.as_ref()))
            .map(|field| field.value)
    }

    fn get_as_str(&self, key: impl AsRef<str>) -> Option<&str> {
        std::str::from_utf8(self.get(key)?).ok()
    }

    fn is_ws_upgrade(&self) -> Option<bool> {
        let upgrade = self
            .get_as_str("upgrade")?
            .eq_ignore_ascii_case("websocket");

        let connection = self
            .get_as_str("connection")?
            .eq_ignore_ascii_case("upgrade");

        Some(upgrade && connection)
    }

    pub fn get_sec_ws_key(&self) -> Option<&[u8]> {
        (self.is_ws_upgrade()? && self.get_as_str("sec-websocket-version")?.contains("13"))
            .then_some(self.get("sec-websocket-key")?)
    }

    pub fn get_sec_ws_accept_key(&self) -> Option<&[u8]> {
        self.is_ws_upgrade()?
            .then_some(self.get("sec-websocket-accept")?)
    }

    pub fn parse_from_raw(&mut self, bytes: &mut &'a [u8]) -> std::result::Result<(), String> {
        *bytes = trim_ascii_start(bytes);

        httparse::parse_headers(bytes, &mut self.inner)
            .map_err(|err| format!("HTTP Error: {err}"))
            .and_then(|status| match status {
                httparse::Status::Partial => Err(format!("HTTP Error: Incomplete http header")),
                httparse::Status::Complete((amt, init)) => {
                    *bytes = &bytes[amt..];
                    self.init = init.len();
                    Ok(())
                }
            })
    }
}

impl<'a, const N: usize> std::fmt::Debug for Header<'a, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Header")
            .field("capacity", &self.inner.len())
            .field("fields", &&self[..])
            .finish()
    }
}

impl<'a, const N: usize> Deref for Header<'a, N> {
    type Target = [httparse::Header<'a>];
    fn deref(&self) -> &Self::Target {
        &self.inner[..self.init]
    }
}

fn trim_ascii_start(mut bytes: &[u8]) -> &[u8] {
    while let [first, rest @ ..] = bytes {
        if matches!(first, b'\t' | b'\n' | b'\x0C' | b'\r' | b' ') {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}
