mod utils;

use std::io::{Error, ErrorKind, Result};
use tokio::{
    io::BufReader,
    net::{TcpListener, TcpStream},
    spawn,
};
use utils::ws;
use web_socket::{CloseCode, CloseEvent, DataType, Event, WebSocket, SERVER};

const ADDR: &str = "127.0.0.1:9002";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("Listening on: {ADDR}");
    let listener = TcpListener::bind(ADDR).await.unwrap();

    while let Ok((stream, _addr)) = listener.accept().await {
        spawn(handle_connection(stream));
    }
}

// ---------------------------------------------------------------------------------------

async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut ws = ws::upgrade(stream).await?;
    ws.on_event = |stream, ev| {
        Box::pin(async move {
            if let Event::Ping(data) = &ev {
                web_socket::send_pong::<SERVER>(stream, data).await?;
            }
            Ok(())
        })
    };

    let event = echo(&mut ws).await.err().unwrap();
    match event.into_inner().unwrap().downcast::<CloseEvent>() {
        Ok(cls_event) => match *cls_event {
            CloseEvent::Error(_) => ws.close(CloseCode::ProtocolError).await?,
            CloseEvent::Close { .. } => ws.close(()).await?,
        },
        Err(_err) => {}
    }
    Ok(())
}

async fn echo(ws: &mut WebSocket<SERVER, BufReader<TcpStream>>) -> Result<()> {
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
