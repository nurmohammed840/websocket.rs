mod utils;

use std::{io::Result, str};
use tokio::net::{TcpListener, TcpStream};
use utils::ws;
use web_socket::{CloseCode, DataType, Event};

const ADDR: &str = "127.0.0.1:9002";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("Listening on: {ADDR}");
    let listener = TcpListener::bind(ADDR).await.unwrap();

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(handle(stream));
    }
}

async fn handle(stream: TcpStream) -> Result<()> {
    let mut ws = ws::upgrade(stream).await?;
    let mut buf = Vec::with_capacity(4096);
    let mut is_txt = true;
    loop {
        match ws.recv().await? {
            Event::Data { ty, done, data } => match (ty, done) {
                (DataType::Text, true) => match str::from_utf8(&data) {
                    Ok(msg) => ws.send(msg).await?,
                    Err(_) => return ws.close(()).await,
                },
                (DataType::Binary, true) => ws.send(&*data).await?,

                // -----------------------------------------------------------
                (DataType::Text | DataType::Binary, false) => {
                    buf.clear();
                    is_txt = ty.is_text();
                    buf.extend_from_slice(&data)
                }
                (DataType::Continue, false) => buf.extend_from_slice(&data),
                (DataType::Continue, true) => {
                    match is_txt {
                        true => match str::from_utf8(&buf) {
                            Ok(msg) => ws.send(msg).await?,
                            Err(_) => return ws.close(()).await,
                        },
                        false => ws.send(&*buf).await?,
                    }
                    buf.clear();
                }
            },
            Event::Ping(data) => ws.send_pong(data).await?,
            Event::Pong(_) => {}
            Event::Error(_) => return ws.close(CloseCode::ProtocolError).await,
            Event::Close { .. } => return ws.close(()).await,
        }
    }
}
