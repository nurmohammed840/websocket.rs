mod utils;
use std::{
    env,
    io::{Error, ErrorKind, Result},
};
use tokio::net::{TcpListener, TcpStream};
use utils::ws;
use web_socket::{DataType, Event};

async fn handeler(stream: TcpStream) -> Result<()> {
    let mut ws = ws::upgrade(stream).await?;
    ws.on_event = Box::new(|ev| {
        Ok(match ev {
            Event::Ping(_) => println!("Ping: {ev}"),
            Event::Pong(_) => println!("Pong: {ev}"),
        })
    });
    loop {
        let mut data = ws.recv().await?;

        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;

        match data.ty {
            DataType::Text => match String::from_utf8(msg) {
                Ok(msg) => {
                    println!("Text: {:?}", msg);
                    ws.send(&*msg).await?;
                }
                Err(err) => return Err(Error::new(ErrorKind::InvalidData, err)),
            },
            DataType::Binary => {
                println!("Binary: {:?}", msg);
                ws.send(&*msg).await?;
            }
        }
    }
}

async fn server(addr: String) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("[Server] Listening at {}", listener.local_addr()?);
    loop {
        let (stream, addr) = listener.accept().await?;
        let error = handeler(stream).await.err().unwrap();
        match error.kind() {
            ErrorKind::NotConnected => println!("Peer {addr} closed successfully."),
            _ => println!("Disconnecting peer {addr}, Cause: {error:#}"),
        }
    }
}

fn main() {
    let mut args = env::args();
    match args.nth(1).as_deref() {
        Some("server" | "-s") => {
            let host = args.next().unwrap_or("0.0.0.0".into());
            let port = host.contains(":").then_some("").unwrap_or(":80");
            let _ = utils::block_on(server(format!("{host}{port}")));
        }
        Some("client" | "-c") => {}
        _ => println!("{HELP}"),
    }
}

const HELP: &str = r#"
USAGE:
    echo server [HOST][:PORT]
    echo client <URI>

Example:
    echo server 127.0.0.1
    echo client ws://localhost:80
"#;
