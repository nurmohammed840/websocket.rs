use crate::frame::*;
use crate::*;

pub mod client;
pub mod server;

pub const SERVER: bool = true;
pub const CLIENT: bool = false;

pub struct Websocket<const SIDE: bool> {
    pub stream: BufReader<TcpStream>,

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
}

impl<const SIDE: bool> Websocket<SIDE> {
    async fn header(&mut self) -> Result<(bool, u8, usize)> {
        loop {
            let [b1, b2] = read_buf(&mut self.stream).await?;

            let fin = b1 & 0b_1000_0000 != 0;
            let opcode = b1 & 0b_1111;
            let len = (b2 & 0b_111_1111) as usize;
            let is_masked = b2 & 0b_1000_0000 != 0;

            if SERVER == SIDE {
                if !is_masked {
                    return err("Expected masked frame");
                }
            } else {
                if is_masked {
                    return err("Expected unmasked frame");
                }
            }

            if opcode >= 8 {
                if !fin || len > 125 {
                    return err("Control frame MUST have a payload length of 125 bytes or less and MUST NOT be fragmented");
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
                    8 => return conn_closed(),
                    // Ping
                    9 => self.send(Pong(&msg)).await?,
                    // Pong
                    10 => {}
                    _ => return err("Unknown opcode"),
                }
            } else {
                if !fin && len == 0 {
                    return err("Fragment length shouldn't be zero");
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
            return err("Expected fragment frame");
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
            _ => return err("Expected data frame"),
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

            #[inline]
            pub async fn recv_next(&mut self) -> Result<bool> {
                if self.ws.len > 0 {
                    return Ok(true);
                }
                match self.ws.fin {
                    true => Ok(false),
                    false => {
                        self._next_frag().await?;
                        Ok(true)
                    }
                }
            }

            #[inline]
            pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
                while !buf.is_empty() {
                    match self.read(buf).await? {
                        0 => break,
                        amt => buf = &mut buf[amt..],
                    }
                }
                match buf.is_empty() {
                    true => Ok(()),
                    false => Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    )),
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
        }
    };
}

pub(super) use default_impl_for_data;
