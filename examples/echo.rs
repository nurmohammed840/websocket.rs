mod utils;
use utils::ws;

use std::io::{ErrorKind, Result};
use tokio::{
    net::{TcpListener, TcpStream},
    spawn,
};
use web_socket::{client::WS, CloseCode, DataType, Event, EventResult};

const USAGE: &str = r#"
USAGE:
    <COMMAND>: Data

COMMAND:
    text, send, msg             Send text message
    ping                        Send ping frame
    pong                        Send pong frame
    quit, exit, close           Close connection

"#;

async fn client(uri: String) -> Result<()> {
    let mut ws = WS::connect(uri, "/").await?;
    println!("[Client] Connected to {}", ws.stream.get_ref().peer_addr()?);

    ws.on_event = Box::new(|ev| on_event(ev, "[ECHO]"));

    let stdin = std::io::stdin();
    let mut buf = String::new();

    let msg = loop {
        buf.clear();
        stdin.read_line(&mut buf)?;
        let (cmd, msg) = buf.split_once(":").unwrap_or(("help", ""));
        let msg = msg.trim();
        match cmd {
            "text" | "send" | "msg" => {
                ws.send(msg).await?;
                println!("[ECHO] Text: {}", ws::read_msg!(ws)?);
            }
            "ping" => ws.send(Event::Ping(msg.as_bytes())).await?,
            "pong" => ws.send(Event::Pong(msg.as_bytes())).await?,
            "quit" | "exit" | "close" => break msg,
            _ => {
                println!("{USAGE}");
                continue;
            }
        }
    };
    ws.close(CloseCode::Normal, msg).await
}

async fn handeler(stream: TcpStream) -> Result<()> {
    let addr = stream.peer_addr()?;
    let mut ws = ws::upgrade(stream).await?;
    ws.on_event = Box::new(move |ev| on_event(ev, &format!("From: {addr}; ")));

    loop {
        let mut data = ws.recv().await?;

        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;

        match data.ty {
            DataType::Text => {
                let msg = String::from_utf8(msg).unwrap();
                println!("From: {addr}; Text: {msg:?}");
                ws.send(&*msg).await?;
            }
            DataType::Binary => {
                println!("From: {addr}; Data: {msg:#?}");
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
        spawn(async move {
            let ev = handeler(stream).await.err().unwrap();
            match ev.kind() {
                ErrorKind::NotConnected => println!("Peer {addr} closed successfully."),
                _ => println!("Disconnecting peer {addr}, Cause: {ev:#}"),
            }
        });
    }
}

fn on_event(ev: Event, pre: &str) -> EventResult {
    match ev {
        Event::Ping(_) => println!("{pre} Ping: {ev}"),
        Event::Pong(_) => println!("{pre} Pong: {ev}"),
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args();
    match args.nth(1).as_deref() {
        Some("server" | "-s" | "--server") => {
            let host = args.next().unwrap_or("0.0.0.0".into());
            let port = host.contains(":").then_some("").unwrap_or(":80");
            let _ = utils::block_on(server(format!("{host}{port}")));
        }
        Some("client" | "-c" | "--client") => {
            let uri = args.next().unwrap_or("ws://localhost:80".into());
            let _ = utils::block_on(client(uri));
        }
        _ => println!("{HELP}"),
    }
}

const HELP: &str = r#"
USAGE:
    echo server [HOST][:PORT]
    echo client [URI]

Example:
    echo server 127.0.0.1
    echo client ws://localhost:80
"#;
