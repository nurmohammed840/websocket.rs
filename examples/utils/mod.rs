#![allow(warnings)]
pub mod handshake;

use httparse::{parse_headers, EMPTY_HEADER};
use web_socket::{WebSocket, CLIENT};
use std::{collections::HashMap, io::Result};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};


pub fn contains<T, U>(opt: &Option<T>, val: U) -> bool
where
    U: PartialEq<T>,
{
    match opt {
        Some(y) => val.eq(y),
        None => false,
    }
}

pub async fn connect(addr: &str, path: &str) -> Result<WebSocket<CLIENT, BufReader<TcpStream>>> {
    let mut stream = BufReader::new(TcpStream::connect(addr).await?);

    // -----------------------------------------------------------

    let (req, sec_key) = handshake::request(addr, path, [("", "")]);
    stream.write_all(req.as_bytes()).await?;

    // -----------------------------------------------------------

    let data = stream.fill_buf().await?;

    if !data.starts_with(b"HTTP/1.1 101 Switching Protocols") {
        panic!("expected upgrade connection");
    }

    let mut headers = [EMPTY_HEADER; 16];
    let (amt, _) = parse_headers(data, &mut headers).unwrap().unwrap();
    let headers: HashMap<_, _> = headers.iter().map(|h| (h.name, h.value)).collect();

    // -----------------------------------------------------------

    if headers
        .get("Sec-WebSocket-Accept")
        .expect("couldn't get `Accept-Key` from http response")
        .ne(&handshake::accept_key_from(sec_key).as_bytes())
    {
        panic!("invalid websocket accept key");
    }
    stream.consume(amt);
    
    Ok(WebSocket::client(stream))
}
