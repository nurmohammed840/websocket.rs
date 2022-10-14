use std::io;
use tokio::{
    io::BufReader,
    net::{TcpStream, ToSocketAddrs},
};

pub const SERVER: bool = true;
pub const CLIENT: bool = false;

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
}

impl Websocket<SERVER> {
    pub fn new(stream: BufReader<TcpStream>) -> Self {
        Self { stream }
    }
}

// use ws_proto::Header;
pub trait Message {
    // fn data_type() -> ws_proto::Opcode;
}

impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
    pub fn send(&mut self, _msg: impl Message) {
        
    }
}

// --------------------------------------------------------------------------

// mod buf_reader;
// mod data;
// mod err;

// use bin_layout::Decoder;
// use buf_reader::BufferReader;
// use std::{
//     array::IntoIter,
//     io::{Error, ErrorKind, Read, Result},
//     iter::Cycle,
//     net::TcpStream,
// };
// use ws_proto::{Header, Opcode};
// pub use data::{Data, DataType};

// impl<const IS_SERVER: bool> Websocket<IS_SERVER> {
//     pub fn recv(&mut self) -> Result<Data> {
//         let (fin, opcode, len, mask) = recv_header(&mut self.stream)?;
//         match opcode {
//             Opcode::Text | Opcode::Binary => Ok(Data {
//                 fin,
//                 len,
//                 mask,
//                 stream: &mut self.stream,
//                 ty: DataType::from(opcode),
//             }),
//             Opcode::Continue => err::proto("Expected data, But got fragment"),
//             Opcode::Ping => todo!(),
//             Opcode::Pong => todo!(),
//             Opcode::Close => todo!(),
//         }
//     }

//     pub fn send(&mut self, _msg: impl Into<String>) {}
// }

// fn recv_header<R: Read>(
//     stream: &mut BufferReader<R>,
// ) -> Result<(bool, Opcode, usize, Cycle<IntoIter<u8, 4>>)> {
//     stream.ensure_data(14)?;
//     let data = stream.buffer();

//     let mut cursor = data.as_ref();
//     let total_len = cursor.len();
//     let Header {
//         fin,
//         opcode,
//         len,
//         mask,
//         ..
//     } = Header::decoder(&mut cursor).map_err(|error| Error::new(ErrorKind::InvalidData, error))?;

//     let mask = match mask {
//         Some(keys) => keys.into_iter().cycle(),
//         None => return err::proto("Got unmasked frame"),
//     };
//     stream.consume(total_len - cursor.len());
//     Ok((fin, opcode, len, mask))
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::net::TcpListener;

//     fn test_main() -> Result<()> {
//         let listener = TcpListener::bind("127.0.0.1:8080")?;

//         for stream in listener.incoming() {
//             let mut ws = Websocket::new(stream?);
//             loop {
//                 let mut data = match ws.recv() {
//                     Ok(data) => data,
//                     // close event
//                     Err(err) if err.kind() == ErrorKind::ConnectionAborted => {
//                         break println!("Socket closed!");
//                     }
//                     // Error event
//                     Err(err) => break println!("Error: {err}"),
//                 };

//                 let mut arr = Vec::new();
//                 data.read_to_end(&mut arr)?;
//             }
//         }
//         Ok(())
//     }
// }
