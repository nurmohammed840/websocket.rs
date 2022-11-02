#![allow(clippy::unusual_byte_groupings)]
use crate::frame::Frame;
use crate::*;

/// client specific implementation
pub mod client;
/// server specific implementation
pub mod server;

/// WebSocket implementation for both client and server
pub struct WebSocket<const SIDE: bool, Stream> {
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

    is_closed: bool,

    fin: bool,
    len: usize,
}

impl<const SIDE: bool, W: Unpin + AsyncWrite> WebSocket<SIDE, W> {
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
    #[inline]
    pub async fn send(&mut self, msg: impl Frame) -> Result<()> {
        let mut bytes = vec![];
        msg.encode::<SIDE>(&mut bytes);
        self.stream.write_all(&bytes).await
    }

    /// Flushes this output stream, ensuring that all intermediately buffered contents reach their destination.
    #[inline]
    pub async fn flash(&mut self) -> Result<()> {
        self.stream.flush().await
    }

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
    pub async fn close(mut self, code: impl Into<u16>, reason: impl AsRef<[u8]>) -> Result<()> {
        self.send(frame::Close {
            code: code.into(),
            reason: reason.as_ref(),
        })
        .await?;
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
                    // Close
                    8 => {
                        let mut writer = vec![];
                        frame::encode::<SIDE, frame::RandMask>(&mut writer, true, 8, &msg);
                        self.stream.write_all(&writer).await?;

                        return err(ErrorKind::NotConnected, "The connection was closed");
                    }
                    // Ping
                    9 => {
                        if let Err((code, reason)) = (self.on_event)(Event::Ping(&msg)) {
                            self.send(frame::Close {
                                code: code as u16,
                                reason: reason.to_string().as_bytes(),
                            })
                            .await?;
                            return err(ErrorKind::Other, reason);
                        };
                        self.send(Event::Pong(&msg)).await?;
                    }
                    // Pong
                    10 => {
                        if let Err((code, reason)) = (self.on_event)(Event::Pong(&msg)) {
                            self.send(frame::Close {
                                code: code as u16,
                                reason: reason.to_string().as_bytes(),
                            })
                            .await?;
                            return err(ErrorKind::Other, reason);
                        }
                    }
                    _ => return proto_err("Unknown opcode"),
                }
            } else {
                if !fin && len == 0 {
                    return proto_err("Fragment length shouldn't be zero");
                }
                let len = match len {
                    126 => u16::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                    127 => u64::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                    len => len,
                };
                return Ok((fin, opcode, len));
            }
        }
    }

    /// After calling this function.
    /// this statement is not possible `self.fin == false && self.len == 0`
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

    #[inline]
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
                $ws.stream.flush().await?;
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
            #[inline]
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

            #[inline]
            pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
                self.read_to_end_with_limit(buf, 8 * 1024 * 1024).await
            }

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

            #[inline]
            pub async fn send(&mut self, data: impl Frame) -> Result<()> {
                self.ws.send(data).await
            }
        }
    };
}

pub(self) use cls_if_err;
pub(self) use default_impl_for_data;
pub(self) use read_exect;
