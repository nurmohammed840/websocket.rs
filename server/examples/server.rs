#![allow(warnings)]
use bin_layout::{Cursor, Decoder, ErrorKind};
use core::slice;
use std::{io, mem::MaybeUninit};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use ws_proto::{handshake, Header, CloseCode};

async fn handler(mut socket: TcpStream) -> io::Result<()> {
    let addr = socket.peer_addr()?;
    println!("Socket Addr: {addr}");
    loop {
        socket.readable().await?;
        let mut buf = [0; 1024];
        match socket.read(&mut buf).await? {
            0 => {
                println!("Client disconnected: {addr}");
                return Ok(());
            }
            v => {
                let mut cursor = Cursor::new(&buf[..v]);
                let header: Result<_, ErrorKind> = Header::decoder(&mut cursor);
                println!("{:#?}", header);
            }
        }
    }
    Ok(())
}

async fn process(mut socket: TcpStream) -> io::Result<()> {
    let mut buf = [0; 1024];
    let n = socket.read(&mut buf).await?;
    let data = &buf[..n];

    if data.starts_with(b"GET /index") {
        socket.write(index_html().as_bytes()).await?;
    }
    if data.starts_with(b"GET /chat HTTP/1.1") {
        let key = "handshake::key(&data).unwrap_or()";
        let res = handshake::response(key);
        socket.write(res.as_bytes()).await?;
        handler(socket).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Goto: http://{}", listener.local_addr()?);
    loop {
        let (socket, addr) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(err) = process(socket).await {
                println!("Addr: {addr}, {err:?}");
            }
        });
    }
}

//======================================================================

fn index_html() -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n{}",
        include_str!("./static/index.html")
    )
}