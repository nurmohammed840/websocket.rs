#![allow(clippy::unusual_byte_groupings)]
use crate::*;

/// client specific implementation
pub mod client;
/// server specific implementation
pub mod server;

/// Used to represent `WebSocket<SERVER>` type.
pub const SERVER: bool = true;
/// Used to represent `WebSocket<CLIENT>` type.
pub const CLIENT: bool = false;

/// WebSocket implementation for both client and server
pub struct WebSocket<const SIDE: bool, Stream> {
    pub stream: Stream,

    /// Listen for incoming websocket [Event].
    ///
    /// ### Example
    ///
    /// ```no_run
    /// use web_socket::{WebSocket, Event};
    /// # async {
    ///
    /// let mut ws = WebSocket::connect("localhost:80", "/").await?;
    /// // Fire when received ping/pong frame.
    /// ws.on_event = Box::new(|ev| {
    ///     println!("{ev:?}");
    ///     Ok(())
    /// });
    ///
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub on_event: Box<dyn FnMut(Event) -> Result<()> + Send + Sync>,

    fin: bool,
    len: usize,
}

impl<const SIDE: bool, W: Unpin + AsyncWrite> WebSocket<SIDE, W> {
    /// ### Example
    ///
    /// ```no_run
    /// use web_socket::{WebSocket, CloseCode, Event};
    /// # async {
    ///
    /// let mut ws = WebSocket::connect("localhost:80", "/").await?;
    /// ws.send("Text Message").await?;
    /// ws.send(b"Binary Data").await?;
    ///
    /// // You can also send control frame.
    /// ws.send(Event::Ping(b"Hello!")).await?;
    /// ws.send(Event::Pong(b"Hello!")).await?;
    ///
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub async fn send(&mut self, msg: impl Frame) -> Result<()> {
        let mut bytes = vec![];
        msg.encode::<SIDE>(&mut bytes);
        self.stream.write_all(&bytes).await
    }

    /// ### Example
    ///
    /// ```no_run
    /// use web_socket::{WebSocket, CloseCode};
    /// # async {
    ///
    /// let ws = WebSocket::connect("localhost:80", "/").await?;
    /// ws.close(CloseCode::Normal, "Closed successfully").await?;
    ///
    /// # std::io::Result::<_>::Ok(()) };
    /// ```
    pub async fn close(mut self, code: impl Into<u16>, reason: impl AsRef<[u8]>) -> Result<()> {
        let code: u16 = code.into();
        let reason = reason.as_ref();

        let mut data = Vec::with_capacity(2 + reason.len());
        data.extend_from_slice(&code.to_be_bytes());
        data.extend_from_slice(reason);

        let mut writer = vec![];
        frame::encode::<SIDE, RandMask>(&mut writer, true, 8, &data);
        self.stream.write_all(&writer).await?;
        self.stream.flush().await
    }
}

impl<const SIDE: bool, Stream> From<Stream> for WebSocket<SIDE, Stream> {
    #[inline]
    fn from(stream: Stream) -> Self {
        Self {
            stream,
            on_event: Box::new(|_| Ok(())),
            fin: true,
            len: 0,
        }
    }
}

impl<const SIDE: bool, RW: Unpin + AsyncBufRead + AsyncWrite> WebSocket<SIDE, RW> {
    async fn header(&mut self) -> Result<(bool, u8, usize)> {
        loop {
            let [b1, b2] = read_buf(&mut self.stream).await?;

            let fin = b1 & 0b_1000_0000 != 0;
            let rsv = b1 & 0b_111_0000;
            let opcode = b1 & 0b_1111;
            let len = (b2 & 0b_111_1111) as usize;
            let is_masked = b2 & 0b_1000_0000 != 0;

            if rsv != 0 {
                return proto_err("Reserve bit MUST be `0`");
            }

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
                        frame::encode::<SIDE, mask::RandMask>(&mut writer, true, 8, &msg);
                        self.stream.write_all(&writer).await?;

                        return err(ErrorKind::NotConnected, "The connection was closed");
                    }
                    // Ping
                    9 => {
                        (self.on_event)(Event::Ping(&msg))?;
                        self.send(Event::Pong(&msg)).await?;
                    }
                    // Pong
                    10 => (self.on_event)(Event::Pong(&msg))?,
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
    [$ws:expr, $code:expr] => {
        match $code {
            Ok(val) => Ok(val),
            Err(err) => {
                $ws.stream.shutdown().await?;
                Err(err)
            }
        }
    };
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

        impl<RW: Unpin + AsyncBufRead + AsyncWrite> Data<'_, RW> {
            #[inline]
            #[allow(clippy::len_without_is_empty)]
            pub fn len(&self) -> usize {
                self.ws.len
            }

            #[inline]
            pub fn fin(&self) -> bool {
                self.ws.fin
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
