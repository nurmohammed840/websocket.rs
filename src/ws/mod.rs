#![allow(clippy::unusual_byte_groupings)]
use crate::*;

// use std::{future::Future, pin::Pin};
// use tokio::io::AsyncWriteExt;

// #[cfg(feature = "client")]
// /// client specific implementation
// pub mod client;

#[cfg(feature = "server")]
/// server specific implementation
pub mod server;

/// WebSocket implementation for both client and server
pub struct WebSocket<const SIDE: bool, Stream> {
    /// it is a low-level abstraction that represents the underlying byte stream over which WebSocket messages are exchanged.
    pub stream: Stream,
    /// used in `cls_if_err`
    _is_closed: bool,
    done: bool
}

impl<const SIDE: bool, W: Unpin + tokio::io::AsyncWrite> WebSocket<SIDE, W> {
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
    ///
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

enum Either<Data> {
    Data(Data),
    Event(Event),
}

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
    async fn header(
        &mut self,
        cb: fn(u8) -> Either<DataType>,
    ) -> Result<Either<(DataType, bool, usize)>> {
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
            return Ok(Either::Event(Event::Error("reserve bit MUST be `0`")));
        }

        // A client MUST mask all frames that it sends to the server. (Note
        // that masking is done whether or not the WebSocket Protocol is running
        // over TLS.)  The server MUST close the connection upon receiving a
        // frame that is not masked.
        //
        // A server MUST NOT mask any frames that it sends to the client.
        if SERVER == SIDE {
            if !is_masked {
                return Ok(Either::Event(Event::Error("expected masked frame")));
            }
        } else if is_masked {
            return Ok(Either::Event(Event::Error("expected unmasked frame")));
        }

        // 3-7 are reserved for further non-control frames.
        if opcode >= 8 {
            if !fin {
                return Ok(Either::Event(Event::Error(
                    "control frame MUST NOT be fragmented",
                )));
            }
            if len > 125 {
                return Ok(Either::Event(Event::Error(
                    "control frame MUST have a payload length of 125 bytes or less",
                )));
            }
            let mut msg = vec![0; len];
            if SERVER == SIDE {
                let mut mask = Mask::from(read_buf(&mut self.stream).await?);
                self.stream.read_exact(&mut msg).await?;
                msg.iter_mut()
                    .zip(&mut mask)
                    .for_each(|(byte, key)| *byte ^= key);
            } else {
                self.stream.read_exact(&mut msg).await?;
            }
            let ev = match opcode {
                // Close
                8 => on_close(msg),
                // Ping
                9 => Event::Ping(msg.into()),
                // Pong
                10 => Event::Pong(msg.into()),
                // 11-15 are reserved for further control frames
                _ => Event::Error("unknown opcode"),
            };
            Ok(Either::Event(ev))
        } else {
            match cb(opcode) {
                Either::Data(data_type) => {
                    let len = match len {
                        126 => u16::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                        127 => u64::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                        len => len,
                    };
                    Ok(Either::Data((data_type, fin, len)))
                }
                Either::Event(ev) => Ok(Either::Event(ev)),
            }
        }
    }

    /// The FIN and opcode fields work together to send a message split up into separate frames. This is called message fragmentation.
    ///
    /// ```txt
    /// Client: FIN=1, opcode=0x1, msg="hello"
    /// Server: (process complete message immediately) Hi.
    /// Client: FIN=0, opcode=0x1, msg="and a"
    /// Server: (listening, new message containing text started)
    /// Client: FIN=0, opcode=0x0, msg="happy new"
    /// Server: (listening, payload concatenated to previous message)
    /// Client: FIN=1, opcode=0x0, msg="year!"
    /// Server: (process complete message) Happy new year to you too!
    /// ```
    ///
    /// ### Note
    ///
    /// - Control frames MAY be injected in the middle of a fragmented message.
    /// - Control frames themselves MUST NOT be fragmented.
    /// - An endpoint MUST be capable of handling control frames in the middle of a fragmented message.
    #[inline]
    async fn next(&mut self) -> Result<Either<(DataType, bool, usize)>> {
        self.header(|opcode| {
            if opcode != 0 {
                return Either::Event(Event::Error("expected fragment frame"));
            }
            Either::Data(DataType::Continue)
        })
        .await
    }

    #[inline]
    async fn _recv(&mut self) -> Result<Either<(DataType, bool, usize)>> {
        self.header(|opcode| match opcode {
            1 => Either::Data(DataType::Text),
            2 => Either::Data(DataType::Binary),
            _ => Either::Event(Event::Error("expected data frame")),
        })
        .await
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

// macro_rules! cls_if_err {
//     [$ws:expr, $code:expr] => ({
//         if $ws.is_closed { err!(NotConnected, "read after close"); }
//         match $code {
//             Ok(val) => Ok(val),
//             Err(err) => {
//                 $ws.is_closed = true;
//                 Err(err)
//             }
//         }
//     });
// }
// macro_rules! read_exect {
//     [$this:expr, $buf:expr, $code:expr] => {
//         loop {
//             match $this._read($buf).await? {
//                 0 => match $buf.is_empty() {
//                     true => break,
//                     false => $code,
//                 },
//                 amt => $buf = &mut $buf[amt..],
//             }
//         }
//     };
// }
// macro_rules! default_impl_for_data {
//     () => {
//         impl<IO: Unpin + AsyncRead> Data<'_, IO> {
//             /// Length of the "Payload data" in bytes.
//             #[inline]
//             #[allow(clippy::len_without_is_empty)]
//             pub fn len(&self) -> usize {
//                 self.ws.len
//             }

//             /// Indicates that this is the final fragment in a message.  The first
//             /// fragment MAY also be the final fragment.
//             #[inline]
//             pub fn fin(&self) -> bool {
//                 self.ws.fin
//             }

//             /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
//             #[inline]
//             pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
//                 cls_if_err!(self.ws, {
//                     if self.ws.len == 0 {
//                         if self.ws.fin {
//                             return Ok(0);
//                         }
//                         self._fragmented_header().await?;
//                     }
//                     self._read(buf).await
//                 })
//             }

//             /// Read the exact number of bytes required to fill buf.
//             #[inline]
//             pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
//                 cls_if_err!(self.ws, {
//                     read_exect!(self, buf, {
//                         if self.ws.fin {
//                             err!(UnexpectedEof, "failed to fill whole buffer");
//                         }
//                         self._fragmented_header().await?;
//                     });
//                     Ok(())
//                 })
//             }

//             /// It is a wrapper around the [Self::read_to_end_with_limit] function with a default limit of `16` MB.
//             #[inline]
//             pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
//                 self.read_to_end_with_limit(buf, 16 * 1024 * 1024).await
//             }

//             /// Reads data until it reaches a specified limit or end of stream.
//             pub async fn read_to_end_with_limit(
//                 &mut self,
//                 buf: &mut Vec<u8>,
//                 limit: usize,
//             ) -> Result<usize> {
//                 cls_if_err!(self.ws, {
//                     let mut amt = 0;
//                     loop {
//                         let additional = self.ws.len;
//                         amt += additional;
//                         if amt > limit {
//                             err!(CloseEvent::Error("data read limit exceeded"));
//                         }
//                         unsafe {
//                             buf.reserve(additional);
//                             let len = buf.len();
//                             let mut uninit = std::slice::from_raw_parts_mut(
//                                 buf.as_mut_ptr().add(len),
//                                 additional,
//                             );
//                             read_exect!(self, uninit, {
//                                 err!(UnexpectedEof, "failed to fill whole buffer");
//                             });
//                             buf.set_len(len + additional);
//                         }
//                         debug_assert!(self.ws.len == 0);
//                         if self.ws.fin {
//                             break Ok(amt);
//                         }
//                         self._fragmented_header().await?;
//                     }
//                 })
//             }
//         }

//         // Re-export
//         impl<IO: Unpin + tokio::io::AsyncWrite> Data<'_, IO> {
//             /// send message to a endpoint by writing it to a WebSocket stream.
//             #[inline]
//             pub async fn send(&mut self, data: impl Message) -> Result<()> {
//                 self.ws.send(data).await
//             }

//             /// Flushes this output stream, ensuring that all intermediately buffered contents reach their destination.
//             #[inline]
//             pub async fn flash(&mut self) -> Result<()> {
//                 self.ws.stream.flush().await
//             }
//         }
//     };
// }

// pub(self) use cls_if_err;
// pub(self) use default_impl_for_data;
// pub(self) use read_exect;
