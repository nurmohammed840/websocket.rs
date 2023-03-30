#![allow(warnings)]
pub mod handshake;

use std::{collections::HashMap, io::Result};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use web_socket::{WebSocket, CLIENT};

macro_rules! io_err {
    [$kind: ident, $msg: expr] => {
        return Err(std::io::Error::new(std::io::ErrorKind::$kind, $msg))
    };
}

#[derive(Debug)]
pub struct Http {
    prefix: String,
    headers: HashMap<String, String>,
}

impl std::ops::Deref for Http {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.headers
    }
}

impl std::ops::DerefMut for Http {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers
    }
}

impl Http {
    pub async fn parse<IO>(reader: &mut IO) -> Result<Self>
    where
        IO: Unpin + AsyncBufRead,
    {
        let mut lines = reader.lines();

        let prefix = lines.next_line().await?.unwrap();
        let mut headers = HashMap::new();

        while let Some(line) = lines.next_line().await? {
            if line == "" {
                break;
            }
            let (key, value) = line.split_once(":").unwrap();
            headers.insert(key.to_ascii_lowercase(), value.trim_start().into());
        }
        Ok(Self { prefix, headers })
    }
}

pub async fn connect(addr: &str, path: &str) -> Result<WebSocket<CLIENT, BufReader<TcpStream>>> {
    let mut stream = BufReader::new(TcpStream::connect(addr).await?);

    let (req, sec_key) = handshake::request(addr, path, [("", "")]);
    stream.write_all(req.as_bytes()).await?;

    let http = Http::parse(&mut stream).await?;

    if !http.prefix.starts_with("HTTP/1.1 101 Switching Protocols") {
        io_err!(InvalidData, "expected upgrade connection");
    }
    if http
        .get("sec-websocket-accept")
        .expect("couldn't get `sec-websocket-accept` from http response")
        .ne(&handshake::accept_key_from(sec_key))
    {
        io_err!(InvalidData, "invalid websocket accept key");
    }

    Ok(WebSocket::client(stream))
}
