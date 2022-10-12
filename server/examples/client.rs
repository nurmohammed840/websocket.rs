use std::{
    io::{Read, Result, Write},
    net::TcpStream,
};

use bin_layout::Encoder;
use ws_proto::{utils::apply_mask, Header, Opcode, Rsv};

fn main() -> Result<()> {
    println!("connection...");
    let mut socket = TcpStream::connect("127.0.0.1:8080")?;
    let res = [
        "GET /chat HTTP/1.1",
        "Host: localhost:8080",
        "Connection: Upgrade",
        "Upgrade: websocket",
        "Sec-WebSocket-Version: 13",
        "Sec-WebSocket-Key: Af8ndUItuOO4E6JcbovlvA==",
        "",
        "",
    ];
    socket.write_all(res.join("\r\n").as_bytes())?;

    let mut buf = [0; 8 * 1024];

    log(&mut socket, &mut buf)?;

    let mut msg = Vec::new();

    Header {
        fin: false,
        rsv: Rsv(0),
        opcode: Opcode::Text,
        len: 5,
        mask: Some([1, 2, 3, 4]),
    }
    .encoder(&mut msg)?;

    let mut hello = b"Hello".to_vec();
    apply_mask([1, 2, 3, 4], &mut hello[..]);
    for item in hello {
        item.encoder(&mut msg)?;
    }

    Header {
        fin: true,
        rsv: Rsv(0),
        opcode: Opcode::Continue,
        len: 5,
        mask: Some([4, 3, 2, 1]),
    }
    .encoder(&mut msg)?;

    let mut hello = b"World".to_vec();
    apply_mask([4, 3, 2, 1], &mut hello[..]);
    for item in hello {
        item.encoder(&mut msg)?;
    }

    socket.write_all(&msg)?;

    // log(&mut socket, &mut buf)?;
    Ok(())
}

fn log(socket: &mut TcpStream, buf: &mut [u8]) -> Result<()> {
    let len = socket.read(buf)?;
    println!("{:?}", String::from_utf8(buf[..len].to_vec()));
    Ok(())
}
