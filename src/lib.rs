mod errors;
mod frame;
mod mask;
mod utils;

pub mod client;
pub mod server;
pub use frame::*;
pub mod handshake;

use errors::*;
use mask::*;
use utils::*;

use std::io::{Error, ErrorKind, Result};
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

impl Websocket<CLIENT> {
    pub async fn connect(addr: impl ToSocketAddrs, request: impl AsRef<str>) -> Result<Self> {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(request.as_ref().as_bytes()).await?;

        let mut stream = BufReader::new(stream);
        let data = stream.fill_buf().await?;

        use handshake::GetSecKey;
        let http_req = std::str::from_utf8(data)
            .map_err(|error| Error::new(ErrorKind::InvalidData, error))?
            .strip_prefix("HTTP/1.1 101 Switching Protocols\r\n")
            .ok_or(Error::new(ErrorKind::InvalidData, "error"))?;

        let headers = handshake::http_headers_from_raw(http_req);

        let _a = headers
            .get_sec_accept_key()
            .ok_or(Error::new(ErrorKind::InvalidData, "error"))?;

            // handshake::sec_accept_key_from(sec_key)

        Ok(Self {
            stream,
            len: 0,
            fin: true,
        })
    }

    pub async fn recv<'a>(&'a mut self) -> Result<client::Data> {
        Ok(client::Data {
            ty: self.read_data_frame_header().await?,
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

    pub async fn recv<'a>(&'a mut self) -> Result<server::Data> {
        let ty = self.read_data_frame_header().await?;
        let mask = Mask::from(read_buf(&mut self.stream).await?);
        Ok(server::Data { ty, mask, ws: self })
    }
}

// --------------------------------------------------------------------------------

impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
    async fn header(&mut self) -> Result<(bool, u8, usize)> {
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
                if IS_SERVER {
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
            if IS_SERVER {
                self.len += 4;
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

// #[cfg(test)]
// mod a {
//     use super::*;
//     use crate::handshake::{get_sec_key, response};
//     use tokio::net::TcpListener;

//     #[tokio::test]
//     async fn main() -> io::Result<()> {
//         let listener = TcpListener::bind("0.0.0.0:8080").await?;
//         println!("Listaning: {}", listener.local_addr()?);

//         let (mut stream, _) = listener.accept().await?;
//         let mut buf = vec![0; 4096];

//         let amt = stream.read(&mut buf).await?;
//         let msg = String::from_utf8(buf[..amt].to_vec()).unwrap();
//         let sec_key = get_sec_key(&msg).unwrap();
//         println!("{:#?}", sec_key);

//         let res = response(sec_key, "");
//         stream.write_all(res.as_bytes()).await?;
//         println!("{}", res);

//         let mut ws = Websocket::new(BufReader::new(stream));

//         let _ = ws.recv().await?;
//         let _ = ws.recv().await?;

//         let mut data = ws.recv().await?;
//         println!("{:?}", data.ty);

//         let mut buf = vec![];
//         data.read_to_end(&mut buf).await?;
//         let msg = String::from_utf8(buf);
//         println!("{:?}", msg);

//         if let Ok(a) = msg {
//             data.send(&*a).await?;
//         }
//         Ok(())
//     }
// }
