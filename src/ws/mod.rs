use crate::frame::*;
use crate::*;

pub mod client;
pub mod server;

pub const SERVER: bool = true;
pub const CLIENT: bool = false;

#[derive(Debug, Clone)]
pub enum Event<'a> {
    Close { code: CloseCode, reason: &'a [u8] },
    Ping(&'a [u8]),
    Pong(&'a [u8]),
}

pub struct Websocket<const SIDE: bool> {
    pub stream: BufReader<TcpStream>,
    pub event: Box<dyn FnMut(Event) -> Result<()>>,

    // this statement is not possible `self.fin == false && self.len == 0`
    fin: bool,
    len: usize,
}

impl<const SIDE: bool> Websocket<SIDE> {
    pub async fn send(&mut self, msg: impl Frame) -> Result<()> {
        let mut bytes = vec![];
        msg.encode::<SIDE>(&mut bytes);
        self.stream.get_mut().write_all(&bytes).await
    }

    pub async fn close(self, code: CloseCode, reason: &[u8]) -> Result<()> {
        let mut bytes = vec![];
        Close { code, reason }.encode::<SIDE>(&mut bytes);
        self.stream.into_inner().write_all(&bytes).await
    }
}

impl<const SIDE: bool> Websocket<SIDE> {
    async fn header(&mut self) -> Result<(bool, u8, usize)> {
        loop {
            let [b1, b2] = read_buf(&mut self.stream).await?;

            let fin = b1 & 0b_1000_0000 != 0;
            let rsv =  b1 & 0b_111_0000;
            let opcode = b1 & 0b_1111;
            let len = (b2 & 0b_111_1111) as usize;
            let is_masked = b2 & 0b_1000_0000 != 0;

            if rsv != 0 {
                return Err(invalid_data("Reserve bit MUST be `0`"));
            }

            if SERVER == SIDE {
                if !is_masked {
                    return Err(invalid_data("Expected masked frame"));
                }
            } else {
                if is_masked {
                    return Err(invalid_data("Expected unmasked frame"));
                }
            }

            if opcode >= 8 {
                if !fin {
                    return Err(invalid_data("Control frame MUST NOT be fragmented"));
                }
                if len > 125 {
                    return Err(invalid_data(
                        "Control frame MUST have a payload length of 125 bytes or less",
                    ));
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
                        let code = CloseCode::try_from(u16::from_be_bytes([msg[0], msg[1]]))
                            .map_err(invalid_data)?;

                        let reason = &msg[2..];
                        (self.event)(Event::Close { code, reason })?;
                        return Err(conn_closed());
                    }
                    // Ping
                    9 => {
                        (self.event)(Event::Ping(&msg))?;
                        self.send(Pong(&msg)).await?;
                    }
                    // Pong
                    10 => (self.event)(Event::Pong(&msg))?,
                    _ => return Err(invalid_data("Unknown opcode")),
                }
            } else {
                if !fin && len == 0 {
                    return Err(invalid_data("Fragment length shouldn't be zero"));
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

    async fn read_fragmented_header(&mut self) -> Result<()> {
        let (fin, opcode, len) = self.header().await?;
        if opcode != 0 {
            return Err(invalid_data("Expected fragment frame"));
        }
        self.fin = fin;
        self.len = len;
        Ok(())
    }

    async fn discard_old_data(&mut self) -> Result<()> {
        loop {
            if self.len > 0 {
                let amt = read_bytes(&mut self.stream, self.len, |_| {}).await?;
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
            _ => return Err(invalid_data("Expected data frame")),
        };

        self.fin = fin;
        self.len = len;
        Ok(data_type)
    }
}

macro_rules! default_impl_for_data {
    () => {
        impl Data<'_> {
            #[inline]
            async fn _has_data(&mut self) -> Result<bool> {
                if self.ws.len == 0 {
                    if self.ws.fin {
                        return Ok(false);
                    }
                    self._next_frag().await?;
                }
                Ok(true)
            }

            #[inline]
            pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
                if self._has_data().await? {
                    return self._read(buf).await;
                }
                Ok(0)
            }

            #[inline]
            pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
                loop {
                    // Let assume `self._read(..)` is expensive, So if the `buf` is empty do nothing.
                    if buf.is_empty() {
                        return Ok(());
                    }
                    match self._read(buf).await? {
                        0 => match buf.is_empty() {
                            true => return Ok(()),
                            false => {
                                if self.ws.fin {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::UnexpectedEof,
                                        "failed to fill whole buffer",
                                    ));
                                }
                                self._next_frag().await?;
                            }
                        },
                        amt => buf = &mut buf[amt..],
                    }
                }
            }

            #[inline]
            pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
                let len = buf.len();
                let additional = self.ws.len;
                buf.reserve(additional);
                unsafe {
                    let end = buf.as_mut_ptr().add(len);
                    let mut uninit = std::slice::from_raw_parts_mut(end, additional);
                    self.read_exact(&mut uninit).await?;
                    buf.set_len(len + additional);
                }
                Ok(additional)
            }

            #[inline]
            pub async fn read_to_end_with_limit(&mut self, _buf: &mut Vec<u8>) -> Result<usize> {
                todo!()
            }
        }

        impl Data<'_> {
            #[inline]
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

pub(super) use default_impl_for_data;
