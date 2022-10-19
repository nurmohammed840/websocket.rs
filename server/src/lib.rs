mod errors;
mod frame;
mod mask;
mod utils;

pub mod client;
pub mod server;
pub use frame::*;

use errors::*;
use mask::*;
use utils::*;

use std::io;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, ToSocketAddrs},
};

pub const SERVER: bool = true;
pub const CLIENT: bool = false;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Text,
    Binary,
}

pub struct Websocket<const IS_SERVER: bool> {
    pub stream: BufReader<TcpStream>,

    // this statement is not possible `self.fin == false && self.len == 0`
    fin: bool,
    len: usize,
}

impl Websocket<CLIENT> {
    pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            stream: BufReader::new(stream),
            len: 0,
            fin: true,
        })
    }

    pub async fn recv<'a>(&'a mut self) -> io::Result<client::Data> {
        let (fin, ty, len) = self.header_data_type().await?;
        Ok(client::Data {
            fin,
            len,
            ty,
            ws: self,
        })
    }
}

impl Websocket<SERVER> {
    pub fn new(stream: BufReader<TcpStream>) -> Self {
        Self {
            stream,
            len: 0,
            fin: true,
        }
    }

    pub async fn recv<'a>(&'a mut self) -> io::Result<server::Data> {
        let (fin, ty, len) = self.header_data_type().await?;
        Ok(server::Data {
            fin,
            ty,
            len,
            mask: Mask::from(read_buf(&mut self.stream).await?),
            ws: self,
        })
    }
}

// --------------------------------------------------------------------------------

impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
    async fn header(&mut self) -> io::Result<(bool, u8, usize)> {
        loop {
            let [b1, b2] = read_buf(&mut self.stream).await?;

            let fin = b1 & 0b_1000_0000 != 0;
            let opcode = b1 & 0b_1111;
            let len = (b2 & 0b_111_1111) as usize;
            let is_masked = b2 & 0b_1000_0000 != 0;

            if IS_SERVER {
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
                self.stream.read_exact(&mut msg).await?;

                if IS_SERVER {
                    let mut mask = Mask::from(read_buf(&mut self.stream).await?);
                    msg.iter_mut()
                        .zip(&mut mask)
                        .for_each(|(byte, key)| *byte ^= key);
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

    async fn discard_old_data(&mut self) -> io::Result<()> {
        loop {
            while self.len > 0 {
                let bytes = self.stream.fill_buf().await?;
                let amt = bytes.len().min(self.len);
                self.len -= amt;
                self.stream.consume(amt);
            }
            if self.fin {
                return Ok(());
            }
            let (fin, opcode, len) = self.header().await?;
            if opcode != 0 {
                return err("Expected fragment frame");
            }
            self.fin = fin;
            self.len = len;
        }
    }

    #[inline]
    async fn header_data_type(&mut self) -> io::Result<(bool, DataType, usize)> {
        self.discard_old_data().await?;

        let (fin, opcode, len) = self.header().await?;
        let data_type = match opcode {
            1 => DataType::Text,
            2 => DataType::Binary,
            _ => return err("Expected data frame"),
        };
        Ok((fin, data_type, len))
    }

    pub async fn send(&mut self, frame: impl Frame) -> io::Result<()> {
        let mut bytes = vec![];
        frame.encode::<IS_SERVER>(&mut bytes);
        self.stream.get_mut().write_all(&bytes).await
    }
}
