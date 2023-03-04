use std::{
    io::{Error, ErrorKind, Result},
    result, str,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use web_socket::{handshake, http, WebSocket, SERVER};

fn handshake_response(request: &mut &[u8]) -> result::Result<String, &'static str> {
    let record = http::Http::parse(request)?;
    let sec_ws_key = record
        .get_sec_ws_key()
        .ok_or("Unable to verify `Sec-WebSocket-Key`")
        .map(|bytes| str::from_utf8(bytes).unwrap())?;

    Ok(handshake::response(sec_ws_key, [("", ""); 0]))
}

pub async fn upgrade(stream: TcpStream) -> Result<WebSocket<SERVER, BufReader<TcpStream>>> {
    // let addr = stream.peer_addr()?;
    let mut stream = BufReader::new(stream);
    let mut data = stream.fill_buf().await?;
    let mut amt = data.len();

    // let req = str::from_utf8(data).map_err(invalid_data)?;
    // println!("[Server] Incoming request from {addr}\n\n{req}");
    let res = handshake_response(&mut data).map_err(invalid_data)?;
    // println!("[Server] Sending response\n\n{res}");

    amt -= data.len();
    stream.consume(amt);
    stream.get_mut().write_all(res.as_bytes()).await?;

    Ok(WebSocket::new(stream))
}

#[macro_export]
macro_rules! read_msg {
    ($ws:expr) => {{
        let mut data = $ws.recv().await?;
        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;
        String::from_utf8(msg).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }};
}

pub use read_msg;

type DynErr = Box<dyn std::error::Error + Send + Sync>;

fn invalid_data(e: impl Into<DynErr>) -> Error {
    Error::new(ErrorKind::InvalidData, e)
}
