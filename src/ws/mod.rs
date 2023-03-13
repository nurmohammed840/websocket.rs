#![allow(clippy::unusual_byte_groupings)]
use crate::*;

#[cfg(feature = "client")]
/// client specific implementation
pub mod client;

mod server;

/// WebSocket implementation for both client and server
pub struct WebSocket<const SIDE: bool, Stream> {
    /// it is a low-level abstraction that represents the underlying byte stream over which WebSocket messages are exchanged.
    pub stream: Stream,
    /// maximum allowed payload length in bytes
    pub max_payload_len: usize,

    /// used in `cls_if_err`
    is_closed: bool,
    done: bool,
}

impl<const SIDE: bool, W: Unpin + AsyncWrite> WebSocket<SIDE, W> {
    /// Send message to a endpoint by writing it to a WebSocket stream.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// use web_socket::{client::WS, CloseCode, Event};
    /// # async {
    ///
    /// let mut ws = WS::connect("localhost:80", "/").await?;
    /// ws.send("Text Message").await?;
    /// ws.send(b"Binary Data").await?;
    ///
    /// // You can also send control frame.
    /// ws.send(Event::Ping(b"Hello!")).await?;
    /// ws.send(Event::Pong(b"Hello!")).await?;
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    #[inline]
    pub async fn send(&mut self, msg: impl Message) -> Result<()> {
        let mut bytes = vec![];
        msg.encode::<SIDE>(&mut bytes);
        self.stream.write_all(&bytes).await
    }

    #[inline]
    ///
    pub async fn send_ping(&mut self, data: impl AsRef<[u8]>) -> Result<()> {
        let mut bytes = vec![];
        message::encode::<SIDE>(&mut bytes, true, 9, data.as_ref());
        self.stream.write_all(&bytes).await
    }

    #[inline]
    /// Sends a pong frame in response to a ping frame received from the WebSocket endpoint.
    pub async fn send_pong(&mut self, data: impl AsRef<[u8]>) -> Result<()> {
        let mut bytes = vec![];
        message::encode::<SIDE>(&mut bytes, true, 10, data.as_ref());
        self.stream.write_all(&bytes).await
    }

    /// Flushes this output stream, ensuring that all intermediately buffered contents reach their destination.
    #[inline]
    pub async fn flash(&mut self) -> Result<()> {
        self.stream.flush().await
    }

    /// - The Close frame MAY contain a body that indicates a reason for closing.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// use web_socket::{client::WS, CloseCode};
    /// # async {
    ///
    /// let ws = WS::connect("localhost:80", "/").await?;
    /// ws.close((CloseCode::Normal, "Closed successfully")).await?;
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub async fn close<T>(mut self, reason: T) -> Result<()>
    where
        T: CloseFrame,
        T::Frame: AsRef<[u8]>,
    {
        self.stream
            .write_all(reason.encode::<SIDE>().as_ref())
            .await?;
        self.stream.flush().await
    }
}

// enum Either<Data> {
//     Data(Data),
//     Event(Event),
// }

impl<const SIDE: bool, IO: Unpin + AsyncRead> WebSocket<SIDE, IO> {
    /// ### WebSocket Frame Header
    ///
    /// ```txt
    ///  0                   1                   2                   3
    ///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    /// +-+-+-+-+-------+-+-------------+-------------------------------+
    /// |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
    /// |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
    /// |N|V|V|V|       |S|             |   (if payload len==126/127)   |
    /// | |1|2|3|       |K|             |                               |
    /// +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
    /// |     Extended payload length continued, if payload len == 127  |
    /// + - - - - - - - - - - - - - - - +-------------------------------+
    /// |                               |Masking-key, if MASK set to 1  |
    /// +-------------------------------+-------------------------------+
    /// | Masking-key (continued)       |          Payload Data         |
    /// +-------------------------------- - - - - - - - - - - - - - - - +
    /// :                     Payload Data continued ...                :
    /// + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
    /// |                     Payload Data continued ...                |
    /// +---------------------------------------------------------------+
    /// ```
    #[inline]
    async fn header<'a, Fut>(
        &'a mut self,
        footer: fn(&'a mut Self, DataType, usize) -> Fut,
    ) -> Result<Event>
    where
        Fut: Future<Output = Result<Event>> + 'a,
    {
        let [b1, b2] = read_buf(&mut self.stream).await?;

        let fin = b1 & 0b_1000_0000 != 0;
        let rsv = b1 & 0b_111_0000;
        let opcode = b1 & 0b_1111;
        let len = (b2 & 0b_111_1111) as usize;

        // Defines whether the "Payload data" is masked.  If set to 1, a
        // masking key is present in masking-key, and this is used to unmask
        // the "Payload data" as per [Section 5.3](https://datatracker.ietf.org/doc/html/rfc6455#section-5.3).  All frames sent from
        // client to server have this bit set to 1.
        let is_masked = b2 & 0b_1000_0000 != 0;

        if rsv != 0 {
            // MUST be `0` unless an extension is negotiated that defines meanings
            // for non-zero values.  If a nonzero value is received and none of
            // the negotiated extensions defines the meaning of such a nonzero
            // value, the receiving endpoint MUST _Fail the WebSocket Connection_.
            err!("reserve bit MUST be `0`");
        }

        // A client MUST mask all frames that it sends to the server. (Note
        // that masking is done whether or not the WebSocket Protocol is running
        // over TLS.)  The server MUST close the connection upon receiving a
        // frame that is not masked.
        //
        // A server MUST NOT mask any frames that it sends to the client.
        if SERVER == SIDE {
            if !is_masked {
                err!("expected masked frame");
            }
        } else if is_masked {
            err!("expected unmasked frame");
        }

        // 3-7 are reserved for further non-control frames.
        if opcode >= 8 {
            if !fin {
                err!("control frame must not be fragmented");
            }
            if len > 125 {
                err!("control frame must have a payload length of 125 bytes or less");
            }
            let mut msg = vec![0; len];
            if SERVER == SIDE {
                let mask = read_buf(&mut self.stream).await?;
                self.stream.read_exact(&mut msg).await?;
                utils::apply_mask(&mut msg, mask);
            } else {
                self.stream.read_exact(&mut msg).await?;
            }
            match opcode {
                // Close
                8 => Ok(on_close(msg)),
                // Ping
                9 => Ok(Event::Ping(msg.into())),
                // Pong
                10 => Ok(Event::Pong(msg.into())),
                // 11-15 are reserved for further control frames
                _ => err!("unknown opcode"),
            }
        } else {
            let data_type = match opcode {
                0 => DataType::Continue,
                1 => DataType::Text,
                2 => DataType::Binary,
                _ => err!("unknown opcode"),
            };
            match (self.done, data_type) {
                (true, DataType::Continue) => err!("expected data frame"),
                (false, DataType::Text | DataType::Binary) => err!("expected fragment frame"),
                _ => self.done = fin,
            }
            let len = match len {
                126 => u16::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                127 => u64::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                len => len,
            };
            if len > self.max_payload_len {
                err!("payload length exceeded");
            }
            footer(self, data_type, len).await
        }
    }
}

/// - If there is a body, the first two bytes of the body MUST be a 2-byte unsigned integer (in network byte order: Big Endian)
///   representing a status code with value /code/ defined in [Section 7.4](https:///datatracker.ietf.org/doc/html/rfc6455#section-7.4).
///   Following the 2-byte integer,
///
/// - The application MUST NOT send any more data frames after sending a `Close` frame.
///
/// - If an endpoint receives a Close frame and did not previously send a
///   Close frame, the endpoint MUST send a Close frame in response.  (When
///   sending a Close frame in response, the endpoint typically echos the
///   status code it received.)  It SHOULD do so as soon as practical.  An
///   endpoint MAY delay sending a Close frame until its current message is
///   sent
///
/// - After both sending and receiving a Close message, an endpoint
///   considers the WebSocket connection closed and MUST close the
///   underlying TCP connection.
fn on_close(msg: Vec<u8>) -> Event {
    let code = msg
        .get(..2)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        .unwrap_or(1000);

    match code {
        1000..=1003 | 1007..=1011 | 1015 | 3000..=3999 | 4000..=4999 => {
            match msg.get(2..).map(|data| String::from_utf8(data.to_vec())) {
                Some(Ok(msg)) => Event::Close {
                    code,
                    reason: msg.into_boxed_str(),
                },
                None => Event::Close {
                    code,
                    reason: "".into(),
                },
                Some(Err(_)) => Event::Error("invalid utf-8 payload"),
            }
        }
        _ => Event::Error("invalid close code"),
    }
}

impl<const SIDE: bool, Stream> fmt::Debug for WebSocket<SIDE, Stream>
where
    Stream: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSocket")
            .field("stream", &self.stream)
            .field("max_payload_len", &self.max_payload_len)
            .field("is_closed", &self.is_closed)
            .finish()
    }
}

impl<const SIDE: bool, IO> From<IO> for WebSocket<SIDE, IO> {
    #[inline]
    fn from(stream: IO) -> Self {
        Self {
            stream,
            max_payload_len: 16 * 1024 * 1024,
            is_closed: false,
            done: true,
        }
    }
}
