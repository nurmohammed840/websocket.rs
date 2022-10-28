#![allow(dead_code)]
use std::{io::Result, thread, time::Duration};

use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::TcpStream,
};
use web_socket::{http, WebSocket, SERVER};

pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

pub async fn sleep(ms: u64) {
    tokio::task::spawn_blocking(move || thread::sleep(Duration::from_millis(ms)))
        .await
        .unwrap();
}

pub async fn upgrade_websocket(
    mut request: &[u8],
    mut stream: TcpStream,
) -> Result<WebSocket<SERVER>> {
    let record = http::Record::from_raw(&mut request).unwrap();
    let sec_ws_key = record
        .get_sec_ws_key()
        .map(std::str::from_utf8)
        .unwrap()
        .unwrap();

    let res = web_socket::handshake::response(sec_ws_key, [("x-server-name", "Fx-WS")]);
    stream.write_all(res.as_bytes()).await?;
    Ok(WebSocket::new(BufReader::new(stream)))
}

#[macro_export]
macro_rules! read_msg {
    ($ws:expr) => ({
        let mut data = $ws.recv().await?;
        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;
        String::from_utf8(msg).unwrap()
    });
}

pub use read_msg;