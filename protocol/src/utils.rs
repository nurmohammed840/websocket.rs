/// # Client handshake request
///
/// A client sends a handshake request to the server. It includes the following information:
///
/// ```yml
/// GET /chat HTTP/1.1
/// Host: example.com:8000
/// Upgrade: websocket
/// Connection: Upgrade
/// Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
/// Sec-WebSocket-Version: 13
/// ```
///
/// The server must be careful to understand everything the client asks for, otherwise security issues can occur.
/// If any header is not understood or has an incorrect value, the server should send a 400 ("Bad Request")} response and immediately close the socket.
///
/// ### Tips
///
/// All browsers send an Origin header.
/// You can use this header for security (checking for same origin, automatically allowing or denying, etc.) and send a 403 Forbidden if you don't like what you see.
/// However, be warned that non-browser agents can send a faked Origin. Most applications reject requests without this header.
///
/// Any http headers is allowed. (Do whatever you want with them)
///
/// ### Note
///
/// - HTTP version must be `1.1` or greater, and method must be `GET`
/// - `Host` header field containing the server's authority.
/// - `Upgrade` header field containing the value `"websocket"`
/// - `Connection` header field that includes the token `"Upgrade"`,
/// - `Sec-WebSocket-Key` header field with a base64-encoded value that, when decoded, is 16 bytes in length.
pub fn sec_web_socket_key(bytes: &[u8]) -> Option<&str> {
    let mut out = None;
    for (k, v) in std::str::from_utf8(bytes).ok()?.lines().filter_map(|line| {
        let mut kv = line.split_terminator(": ");
        kv.next().zip(kv.next())
    }) {
        if (k.eq_ignore_ascii_case("upgrade") && v != "websocket")
            | (k.eq_ignore_ascii_case("connection") && v != "Upgrade")
            | (k.eq_ignore_ascii_case("sec-websocket-version") && v != "13")
        {
            return None;
        }
        if k.eq_ignore_ascii_case("sec-websocket-key") {
            out = Some(v);
        }
    }
    out
}

/// ## Server handshake response
///
/// When the server receives the handshake request,
/// It should send back a special response that indicates that the protocol will be changing from HTTP to WebSocket.
///
/// The Sec-WebSocket-Accept header is important in that the server must derive it from the `Sec-WebSocket-Key` that the client sent to it.
///
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
/// assert_eq!(handshake_res("dGhlIHNhbXBsZSBub25jZQ=="), res.join("\r\n"));
/// ```
///
/// To get it, concatenate the client's `Sec-WebSocket-Key` and the string _"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"_ together (it's a [Magic string](https://en.wikipedia.org/wiki/Magic_string)), take the SHA-1 hash of the result, and return the base64 encoding of that hash.
///
///
/// 1. If the connection is happening on an HTTPS (HTTP-over-TLS) port,
///    perform a TLS handshake over the connection.  If this fails
///    (e.g., the client indicated a host name in the extended client
///    hello "server_name" extension that the server does not host),
///    then close the connection.
///
/// 2. The server can perform additional client authentication, Or The server MAY redirect the client.
///
///
/// ### Note
///
/// - Regular HTTP status codes can be used only before the handshake. After the handshake succeeds, you have to use a different set of codes (defined in section 7.4 of the spec)
pub fn handshake_response(sec_web_socket_key: &str) -> String {
    use sha1::{Digest, Sha1};
    let mut m = Sha1::new();
    m.update(sec_web_socket_key.as_bytes());
    m.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"); // Magic string
    let key = base64::encode(m.finalize());

    format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {key}\r\n\r\n",)
}

pub fn apply_mask<const S: usize>(keys: [u8; S], payload: &mut [u8]) {
    payload
        .iter_mut()
        .zip(keys.into_iter().cycle())
        .for_each(|(p, m)| *p ^= m);
}
