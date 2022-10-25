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
        // *bytes = trim_ascii_start(bytes);

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

#[cfg(test)]
mod tests {
    use std::str;

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

    pub fn trim_ascii_end(mut bytes: &[u8]) -> &[u8] {
        while let [rest @ .., last] = bytes {
            if last.is_ascii_whitespace() {
                bytes = rest;
            } else {
                break;
            }
        }
        bytes
    }

    /// ```rust
    /// let mut bytes = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n".as_bytes();
    /// let header = HttpRecord::from_raw(&mut bytes).unwrap();
    ///
    /// assert_eq!(header.schema, "HTTP/1.1 101 Switching Protocols".as_bytes());
    /// assert_eq!(header.get("upgrade"), Some("websocket".as_bytes()));
    /// assert_eq!(
    ///     header.get_sec_ws_accept_key(),
    ///     Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".as_bytes())
    /// );
    /// ```rust
    #[derive(Default, Clone)]
    pub struct HttpRecord<'a> {
        pub schema: &'a [u8],
        pub header: Vec<(&'a [u8], &'a [u8])>,
    }

    impl<'a> std::fmt::Debug for HttpRecord<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut header = vec![];
            for (key, value) in &self.header {
                match (str::from_utf8(key), str::from_utf8(value)) {
                    (Ok(key), Ok(value)) => header.push((key, value)),
                    _ => {}
                }
            }
            f.debug_struct("HttpRecord")
                .field("schema", &str::from_utf8(self.schema))
                .field("header", &header)
                .finish()
        }
    }

    const HTTP_EOF_ERR: &str = "HTTP parse error: Unexpected end";

    impl<'a> HttpRecord<'a> {
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

        pub fn get_sec_ws_accept_key(&self) -> Option<&[u8]> {
            self.is_ws_upgrade()?
                .then_some(self.get("sec-websocket-accept")?)
        }

        fn from_raw(bytes: &mut &'a [u8]) -> std::result::Result<Self, &'static str> {
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

    fn split_once<'a>(reader: &mut &'a [u8], ascii: u8) -> Option<&'a [u8]> {
        let index = reader.iter().position(|&byte| byte == ascii)?;
        let val = &reader[..index];
        *reader = &reader[index + 1..];
        Some(val)
    }

    #[test]
    fn test_name() {
        // let record = HttpRecord::from_raw(&mut bytes).unwrap();
        // println!("{:?}", record);
    }
}
