mod errors;
mod utils;

pub mod client;
pub mod server;

use errors::*;

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
        let (_fin, ty, len) = self.header_data_type().await?;
        let _mask: [u8; 4] = read_buf(&mut self.stream).await?;
        Ok(server::Data { len, ty, ws: self })
    }
}

// --------------------------------------------------------------------------------

impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
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
                8 => todo!(),
                // Ping
                9 => todo!(),
                // Pong
                10 => todo!(),
                _ => return err("Unknown opcode"),
            }
        } else {
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
            _ => return err("Expected data, But got fragment"),
        };
        Ok((fin, data_type, len))
    }
}

// --------------------------------------------------------------------------------

// impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
//     pub async fn read(&mut self) -> Result<Vec<u8>, DynErr> {
//         use ws_proto::Opcode::*;
//         let mut data: Vec<u8> = vec![];

//         let (_fin, opcode, len, mask) = self.read_header().await?;

//         if IS_SERVER {
//             let mask = mask.unwrap();

//             data.reserve(len);
//             let remaining = data.as_mut_ptr();
//             let mut i = 0;

//             self.read_upto(len, |slice| {
//                 for byte in slice {
//                     unsafe { *remaining.add(i) = byte ^ mask.get_unchecked(i % 4) };
//                     i += 1;
//                 }
//             })
//             .await?;
//             unsafe { data.set_len(i) };
//         } else {
//             self.read_upto(len, |slice| data.extend_from_slice(slice))
//                 .await?;
//         }

//         match opcode {
//             Continue => return Err("Expected data, But got fragment".into()),
//             Text | Binary => {}
//             Close => {}
//             Ping => {}
//             Pong => {}
//         }
//         Ok(data)
//     }

//     async fn read_upto(&mut self, mut len: usize, mut cb: impl FnMut(&[u8])) -> io::Result<()> {
//         while len > 0 {
//             let buf = self.stream.fill_buf().await?;
//             let amt = buf.len().min(len);
//             cb(unsafe { buf.get_unchecked(..amt) });
//             self.stream.consume(amt);
//             len -= amt;
//         }
//         Ok(())
//     }

//     async fn fragment(&mut self, data: &mut Vec<u8>) -> Result<(), DynErr> {
//         let (_fin, opcode, len, mask) = self.read_header().await?;
//         match opcode {
//             ws_proto::Opcode::Continue => {

//             },
//             ws_proto::Opcode::Text | ws_proto::Opcode::Binary => {
//                 return Err("Expected fragment, But got data".into())
//             }
//             ws_proto::Opcode::Close => {},
//             ws_proto::Opcode::Ping => {},
//             ws_proto::Opcode::Pong => {},
//         }
//         Ok(())
//     }
// }

// //---------------------------------------------------------------------------------------------

async fn read_buf<const N: usize>(stream: &mut BufReader<TcpStream>) -> io::Result<[u8; N]> {
    let mut buf = [0; N];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

// impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
//     async fn read_header(
//         &mut self,
//     ) -> Result<(bool, ws_proto::Opcode, usize, Option<[u8; 4]>), DynErr> {
//         use ws_proto::Opcode::*;
//         let [b1, b2] = read_buf(&mut self.stream).await?;
//         let fin = b1 & 0b_1000_0000 != 0;
//         let opcode = match b1 & 0b_1111 {
//             0 => Continue,
//             1 => Text,
//             2 => Binary,
//             8 => Close,
//             9 => Ping,
//             10 => Pong,
//             _ => return Err("Unknown opcode".into()),
//         };
//         let len = b2 & 0b_111_1111;
//         let len = if opcode.is_control() {
//             if !fin || len > 125 {
//                 return Err("Control frames MUST have a payload length of 125 bytes or less and MUST NOT be fragmented".into());
//             }
//             len as usize
//         } else {
//             match len {
//                 126 => u16::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
//                 127 => u64::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
//                 len => len as usize,
//             }
//         };
//         let mask = if b2 & 0b_1000_0000 != 0 {
//             Some(read_buf(&mut self.stream).await?)
//         } else {
//             None
//         };
//         Ok((fin, opcode, len, mask))
//     }
// }
