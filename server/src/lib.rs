mod errors;
mod utils;

pub mod client;
pub mod server;

use errors::*;
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
            mask: server::Mask::new(read_buf(&mut self.stream).await?),
            ws: self,
        })
    }
}

// --------------------------------------------------------------------------------

impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
    // async fn get_msg(&mut self, len: usize, writer: &mut Vec<u8>) -> io::Result<()> {
    //     if IS_SERVER {
    //     } else {
    //     }
    //     Ok(())
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
        let _ = Self::encode_frame;
    }

    #[inline]
    fn encode_frame(writer: &mut Vec<u8>, fin: bool, opcode: u8, mask: [u8; 4], data: &[u8]) {
        let data_len = data.len();
        writer.reserve(if IS_SERVER { 10 } else { 14 } + data_len);
        unsafe {
            let filled = writer.len();
            let start = writer.as_mut_ptr().add(filled);

            let mask_bit = if IS_SERVER { 0 } else { 0x80 };

            start.write(((fin as u8) << 7) | opcode);
            let len = if data_len < 126 {
                start.add(1).write(mask_bit | data_len as u8);
                2
            } else if data_len < 65536 {
                let [b2, b3] = (data_len as u16).to_be_bytes();
                start.add(1).write(mask_bit | 126);
                start.add(2).write(b2);
                start.add(3).write(b3);
                4
            } else {
                let [b2, b3, b4, b5, b6, b7, b8, b9] = (data_len as u64).to_be_bytes();
                start.add(1).write(mask_bit | 127);
                start.add(2).write(b2);
                start.add(3).write(b3);
                start.add(4).write(b4);
                start.add(5).write(b5);
                start.add(6).write(b6);
                start.add(7).write(b7);
                start.add(8).write(b8);
                start.add(9).write(b9);
                10
            };

            let header_len = if IS_SERVER {
                std::ptr::copy_nonoverlapping(data.as_ptr(), start.add(len), data_len);
                len
            } else {
                let [a, b, c, d] = mask;
                start.add(len).write(a);
                start.add(len + 1).write(b);
                start.add(len + 2).write(c);
                start.add(len + 3).write(d);

                let dist = start.add(len + 4);
                for (index, byte) in data.iter().enumerate() {
                    dist.add(index).write(byte ^ mask[index % 4]);
                }
                len + 4
            };
            writer.set_len(filled + header_len + data_len);
        }
        // encoded
    }
}

#[cfg(test)]
mod encode {
    use super::*;
    const DATA: &[u8] = b"Hello";

    #[test]
    fn unmasked_txt_msg() {
        let mut bytes = vec![];
        Websocket::<SERVER>::encode_frame(&mut bytes, true, 1, [0; 4], DATA);
        assert_eq!(bytes, [0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn masked_txt_msg() {
        let mut bytes = vec![];
        Websocket::<CLIENT>::encode_frame(&mut bytes, true, 1, [55, 250, 33, 61], DATA);
        assert_eq!(
            bytes,
            [0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58]
        );
    }

    #[test]
    fn fragmented_unmasked_txt_msg() {
        let mut bytes = vec![];
        Websocket::<SERVER>::encode_frame(&mut bytes, false, 1, [0; 4], b"Hel");
        Websocket::<SERVER>::encode_frame(&mut bytes, true, 0, [0; 4], b"lo");
        assert_eq!(
            bytes,
            [
                0x01, 0x03, 0x48, 0x65, 0x6c, // fragmented frame
                0x80, 0x02, 0x6c, 0x6f, // final frame
            ]
        );
    }

    #[test]
    fn unmasked_ping_req_and_masked_pong_res() {
        let mut bytes = vec![];
        Websocket::<SERVER>::encode_frame(&mut bytes, true, 9, [0; 4], DATA);
        Websocket::<CLIENT>::encode_frame(&mut bytes, true, 10, [55, 250, 33, 61], DATA);
        assert_eq!(
            bytes,
            [
                // unmasked ping request
                0x89, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, //
                // masked pong response
                0x8a, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
            ]
        );
    }
}
