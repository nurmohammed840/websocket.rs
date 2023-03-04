mod utils;

use std::io::{Error, ErrorKind, Result};
use tokio::{
    net::{TcpListener, TcpStream},
    spawn,
};
use utils::ws;
use web_socket::{CloseCode, DataType, WebSocket, SERVER};

const ADDR: &str = "127.0.0.1:9002";

#[tokio::main]
async fn main() {
    println!("Listening on: {ADDR}");
    let listener = TcpListener::bind(ADDR).await.unwrap();

    while let Ok((stream, _addr)) = listener.accept().await {
        spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut ws = ws::upgrade(stream).await?;
    match echo(&mut ws).await.err().unwrap().kind() {
        ErrorKind::NotConnected | ErrorKind::InvalidData => Ok(()),
        _ => ws.close(CloseCode::ProtocolError).await,
    }
}

async fn echo(ws: &mut WebSocket<SERVER, tokio::io::BufReader<TcpStream>>) -> Result<()> {
    loop {
        let mut data = ws.recv().await?;

        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;

        match data.ty {
            DataType::Binary => ws.send(&*msg).await?,
            DataType::Text => {
                let msg = std::str::from_utf8(&msg)
                    .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

                ws.send(msg).await?;
            }
        }
    }
}
