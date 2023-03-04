#![allow(clippy::unusual_byte_groupings)]
use crate::*;

/// client specific implementation
pub mod client;
/// server specific implementation
pub mod server;

/// WebSocket implementation for both client and server
pub struct WebSocket<const SIDE: bool, Stream> {
    /// it is a low-level abstraction that represents the underlying byte stream over which WebSocket messages are exchanged.
    pub stream: Stream,

    /// Listen for incoming websocket [Event].
    ///
    /// ### Example
    ///
    /// ```no_run
    /// use web_socket::{client::WS, Event};
    /// # async {
    ///
    /// let mut ws = WS::connect("localhost:80", "/").await?;
    /// // Fire when received ping/pong frame.
    /// ws.on_event = Box::new(|ev| {
    ///     println!("{ev:?}");
    ///     Ok(())
    /// });
    ///
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub on_event: Box<dyn FnMut(Event) -> EventResult + Send + Sync>,

    /// See: [cls_if_err]
    is_closed: bool,

    fin: bool,
    len: usize,
}

impl<const SIDE: bool, W: Unpin + AsyncWrite> WebSocket<SIDE, W> {
    /// send message to a endpoint by writing it to a WebSocket stream.
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
    ///
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub async fn send(&mut self, msg: impl Message) -> Result<()> {
        let mut bytes = vec![];
        msg.encode::<SIDE>(&mut bytes);
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
    /// ws.close(CloseCode::Normal, "Closed successfully").await?;
    ///
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub async fn close(mut self, reason: impl CloseReason) -> Result<()> {
        let mut bytes = vec![];
        reason.encode::<SIDE>(&mut bytes);
        self.stream.write_all(&bytes).await?;

        self.stream.flush().await
    }
}

impl<const SIDE: bool, Stream> From<Stream> for WebSocket<SIDE, Stream> {
    #[inline]
    fn from(stream: Stream) -> Self {
        Self {
            stream,
            on_event: Box::new(|_| Ok(())),

            is_closed: false,

            fin: true,
            len: 0,
        }
    }
}

impl<const SIDE: bool, RW: Unpin + AsyncBufRead + AsyncWrite> WebSocket<SIDE, RW> {
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
    async fn header(&mut self) -> Result<(bool, u8, usize)> {
        loop {
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
                return proto_err("Reserve bit MUST be `0`");
            }

            // A client MUST mask all frames that it sends to the server. (Note
            // that masking is done whether or not the WebSocket Protocol is running
            // over TLS.)  The server MUST close the connection upon receiving a
            // frame that is not masked.
            //
            // A server MUST NOT mask any frames that it sends to the client.
            if SERVER == SIDE {
                if !is_masked {
                    return proto_err("Expected masked frame");
                }
            } else if is_masked {
                return proto_err("Expected unmasked frame");
            }

            if opcode >= 8 {
                if !fin {
                    return proto_err("Control frame MUST NOT be fragmented");
                }
                if len > 125 {
                    return proto_err(
                        "Control frame MUST have a payload length of 125 bytes or less",
                    );
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

                match opcode {
                    // 3-7 are reserved for further non-control frames.

                    // Close
                    8 => {
                        // - If there is a body, the first two bytes of the body MUST be a 2-byte unsigned integer (in network byte order: Big Endian)
                        //   representing a status code with value /code/ defined in [Section 7.4](https://datatracker.ietf.org/doc/html/rfc6455#section-7.4). Following the 2-byte integer,
                        //
                        // - Close frames sent from client to server must be masked.
                        // - The application MUST NOT send any more data frames after sending a `Close` frame.
                        //
                        // - If an endpoint receives a Close frame and did not previously send a
                        //   Close frame, the endpoint MUST send a Close frame in response.  (When
                        //   sending a Close frame in response, the endpoint typically echos the
                        //   status code it received.)  It SHOULD do so as soon as practical.  An
                        //   endpoint MAY delay sending a Close frame until its current message is
                        //   sent
                        //
                        // - After both sending and receiving a Close message, an endpoint
                        //   considers the WebSocket connection closed and MUST close the
                        //   underlying TCP connection.

                        // Feature: Do we really need to check invalid UTF8 (`msg`) payload ? Maybe not...
                        if let Some(1000..=1003 | 1007..=1011 | 1015) = msg
                            .get(..2)
                            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
                        {
                            let mut writer = vec![];
                            message::encode::<SIDE>(&mut writer, true, 8, &msg);
                            let _ = self.stream.write_all(&writer).await;
                        }
                        return err(ErrorKind::NotConnected, "The connection was closed");
                    }
                    // Ping
                    9 => {
                        // A Ping frame MAY include "Application data".
                        // Unless it already received a Close frame.  It SHOULD respond with Pong frame as soon as is practical.
                        //
                        // A Ping frame may serve either as a keepalive or as a means to verify that the remote endpoint is still responsive.
                        if let Err(reason) = (self.on_event)(Event::Ping(&msg)) {
                            // let _ = self
                            //     .send(message::Close {
                            //         code: code as u16,
                            //         reason: reason.to_string().as_bytes(),
                            //     })
                            //     .await;
                            return err(ErrorKind::Other, reason);
                        };
                        self.send(Event::Pong(&msg)).await?;
                    }
                    // Pong
                    10 => {
                        // A Pong frame sent in response to a Ping frame must have identical
                        // "Application data" as found in the message body of the Ping frame being replied to.
                        //
                        // If an endpoint receives a Ping frame and has not yet sent Pong frame(s) in response to previous Ping frame(s), the endpoint MAY
                        // elect to send a Pong frame for only the most recently processed Ping frame.
                        //
                        //  A Pong frame MAY be sent unsolicited.  This serves as a unidirectional heartbeat.  A response to an unsolicited Pong frame is not expected.
                        if let Err(reason) = (self.on_event)(Event::Pong(&msg)) {
                            // let _ = self
                            //     .send(message::Close {
                            //         code: code as u16,
                            //         reason: reason.to_string().as_bytes(),
                            //     })
                            //     .await;
                            return err(ErrorKind::Other, reason);
                        }
                    }
                    // 11-15 are reserved for further control frames
                    _ => return proto_err("Unknown opcode"),
                }
            } else {
                // Feature: client may intentionally sends consecutive fragment frames of size `0` ?
                // if !fin && len == 0 {
                //     return proto_err("Fragment length shouldn't be zero");
                // }
                let len = match len {
                    126 => u16::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                    127 => u64::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                    len => len,
                };
                return Ok((fin, opcode, len));
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
    /// - Control frames MAY be injected in the middle ofa fragmented message.
    ///   Control frames themselves MUST NOT be fragmented.
    ///   An endpoint MUST be capable of handling control frames in the middle of a fragmented message.
    async fn read_fragmented_header(&mut self) -> Result<()> {
        let (fin, opcode, len) = self.header().await?;
        if opcode != 0 {
            return proto_err("Expected fragment frame");
        }
        self.fin = fin;
        self.len = len;
        Ok(())
    }

    async fn discard_old_data(&mut self) -> Result<()> {
        loop {
            if self.len > 0 {
                let amt = read_bytes(&mut self.stream, self.len, |_| {}).await?;
                debug_assert!(amt != 0);
                self.len -= amt;
                continue;
            }
            if self.fin {
                return Ok(());
            }
            self.read_fragmented_header().await?;
            // also skip masking keys sended from client
            if SERVER == SIDE {
                self.len += 4;
            }
        }
    }

    async fn read_data_frame_header(&mut self) -> Result<DataType> {
        self.discard_old_data().await?;

        let (fin, opcode, len) = self.header().await?;
        let data_type = match opcode {
            1 => DataType::Text,
            2 => DataType::Binary,
            _ => return proto_err("Expected data frame"),
        };

        self.fin = fin;
        self.len = len;
        Ok(data_type)
    }
}

macro_rules! cls_if_err {
    [$ws:expr, $code:expr] => ({
        if $ws.is_closed {
            return err(ErrorKind::NotConnected, "Read after close");
        }
        match $code {
            Ok(val) => Ok(val),
            Err(err) => {
                $ws.is_closed = true;
                let _ = $ws.stream.flush().await;
                Err(err)
            }
        }
    });
}
macro_rules! read_exect {
    [$this:expr, $buf:expr, $code:expr] => {
        loop {
            match $this._read($buf).await? {
                0 => match $buf.is_empty() {
                    true => break,
                    false => $code,
                },
                amt => {
                    $buf = &mut $buf[amt..];
                    // Let assume `self._read(..)` is expensive, So if the `buf` is empty do nothing.
                    if $buf.is_empty() { break }
                },
            }
        }
    };
}

macro_rules! default_impl_for_data {
    () => {
        impl<RW: Unpin + AsyncBufRead + AsyncWrite> Data<'_, RW> {
            /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
            pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
                cls_if_err!(self.ws, {
                    if self.len() == 0 {
                        if self.ws.fin {
                            return Ok(0);
                        }
                        self._read_next_frag().await?;
                    }
                    self._read(buf).await
                })
            }

            /// Read the exact number of bytes required to fill buf.
            pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
                cls_if_err!(self.ws, {
                    Ok(read_exect!(self, buf, {
                        if self.fin() {
                            return err(ErrorKind::UnexpectedEof, "failed to fill whole buffer");
                        }
                        self._read_next_frag().await?;
                    }))
                })
            }

            /// It is a wrapper around the [Self::read_to_end_with_limit] function with a default limit of `16` MB.
            #[inline]
            pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
                self.read_to_end_with_limit(buf, 16 * 1024 * 1024).await
            }

            /// Reads data until it reaches a specified limit or end of stream.
            pub async fn read_to_end_with_limit(
                &mut self,
                buf: &mut Vec<u8>,
                limit: usize,
            ) -> Result<usize> {
                cls_if_err!(self.ws, {
                    let mut amt = 0;
                    loop {
                        let additional = self.len();
                        amt += additional;
                        if amt > limit {
                            return err(ErrorKind::Other, "Data read limit exceeded");
                        }
                        unsafe {
                            buf.reserve(additional);
                            let len = buf.len();
                            let mut uninit = std::slice::from_raw_parts_mut(
                                buf.as_mut_ptr().add(len),
                                additional,
                            );
                            read_exect!(self, uninit, {
                                return err(
                                    ErrorKind::UnexpectedEof,
                                    "failed to fill whole buffer",
                                );
                            });
                            buf.set_len(len + additional);
                        }
                        debug_assert!(self.len() == 0);
                        if self.fin() {
                            break Ok(amt);
                        }
                        self._read_next_frag().await?;
                    }
                })
            }
        }

        // Re-export
        impl<RW: Unpin + AsyncBufRead + AsyncWrite> Data<'_, RW> {
            /// Length of the "Payload data" in bytes.
            #[inline]
            #[allow(clippy::len_without_is_empty)]
            pub fn len(&self) -> usize {
                self.ws.len
            }

            /// Indicates that this is the final fragment in a message.  The first
            /// fragment MAY also be the final fragment.
            #[inline]
            pub fn fin(&self) -> bool {
                self.ws.fin
            }

            /// Flushes this output stream, ensuring that all intermediately buffered contents reach their destination.
            #[inline]
            pub async fn flash(&mut self) -> Result<()> {
                self.ws.stream.flush().await
            }

            /// send message to a endpoint by writing it to a WebSocket stream.
            #[inline]
            pub async fn send(&mut self, data: impl Message) -> Result<()> {
                self.ws.send(data).await
            }
        }
    };
}

pub(self) use cls_if_err;
pub(self) use default_impl_for_data;
pub(self) use read_exect;
