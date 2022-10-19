mod errors;
mod frame;
mod utils;
mod mask;

pub mod client;
pub mod server;
pub use frame::{Frame, Ping};

use errors::*;
use mask::*;
use utils::*;

use std::io;
use tokio::{
    io::{AsyncReadExt, BufReader},
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
}

impl Websocket<CLIENT> {
    pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            stream: BufReader::new(stream),
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
        Self { stream }
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
    // async fn get_msg(&mut self, len: usize, writer: &mut Vec<u8>) -> io::Result<()> {
    // }

    async fn header(&mut self) -> io::Result<(bool, u8, usize)> {
        let Self { stream } = self;
        let [b1, b2] = read_buf(stream).await?;

        let fin = b1 & 0b_1000_0000 != 0;
        let opcode = b1 & 0b_1111;
        let len = b2 & 0b_111_1111;
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
            match opcode {
                // Close
                8 => {
                    todo!()
                }
                // Ping
                9 => {
                    let mut _res: Vec<u8> = Vec::with_capacity(len as usize);

                    todo!()
                }
                // Pong
                10 => {
                    todo!()
                }
                _ => return err("Unknown opcode"),
            }
        } else {
            if !fin && len == 0 {
                return err("Fragment length shouldn't be zero");
            }
            let len = match len {
                126 => u16::from_be_bytes(read_buf(stream).await?) as usize,
                127 => u64::from_be_bytes(read_buf(stream).await?) as usize,
                len => len as usize,
            };
            Ok((fin, opcode, len))
        }
    }

    #[inline]
    async fn header_data_type(&mut self) -> io::Result<(bool, DataType, usize)> {
        let (fin, opcode, len) = self.header().await?;
        let data_type = match opcode {
            1 => DataType::Text,
            2 => DataType::Binary,
            _ => return err("Expected data frame"),
        };
        Ok((fin, data_type, len))
    }

    pub fn send(_data: &[u8]) {
        // let _ = Self::encode_frame;
    }
}
