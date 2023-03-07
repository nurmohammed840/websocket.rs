//! Run server: `cargo run --example echo -- server`
//! Run client: `cargo run --example echo -- client`

mod utils;
use utils::ws;

use std::io::Result;
use tokio::{
    net::{TcpListener, TcpStream},
    spawn,
};
use web_socket::{client::WS, CloseCode, CloseEvent, DataType, Event};

const HELP: &str = r#"
USAGE:
    echo server [HOST][:PORT]
    echo client [URI]

Example:
    echo server 127.0.0.1
    echo client localhost:80
"#;

const USAGE: &str = r#"
______________________________________________________
|                                                    |
| USAGE: <COMMAND>: <data>                           |
|                                                    |
| COMMAND:                                           |
|    text, send                  Send text message   |
|    ping                        Send ping frame     |
|    pong                        Send pong frame     |
|    q, quit, exit               Close connection    |
|____________________________________________________|"#;

async fn server(addr: String) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("[Server] Listening at {}", listener.local_addr()?);
    loop {
        let (stream, addr) = listener.accept().await?;
        spawn(async move {
            let ev = handeler(stream).await.err().unwrap();
            match ev.into_inner().unwrap().downcast::<CloseEvent>() {
                Ok(cls_event) => match *cls_event {
                    CloseEvent::Error(err) => {
                        println!("Disconnecting peer {addr}, Cause: {err:#?}")
                    }
                    CloseEvent::Close { .. } => println!("Peer {addr} closed successfully."),
                },
                Err(io_err) => println!("Error: {io_err:#?}"),
            }
        });
    }
}

async fn handeler(stream: TcpStream) -> Result<()> {
    let addr = stream.peer_addr()?;
    let mut ws = ws::upgrade(stream).await?;
    ws.on_event = Box::new(move |ev| {
        on_event(ev, &format!("From: {addr}; "));
        Ok(())
    });

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

async fn client(uri: String) -> Result<()> {
    let mut ws = WS::connect(uri, "/").await?;
    println!("[Client] Connected to {}", ws.stream.get_ref().peer_addr()?);

    ws.on_event = Box::new(|ev| {
        on_event(ev, "[ECHO]");
        Ok(())
    });

    let stdin = std::io::stdin();
    let mut buf = String::new();

    let msg = loop {
        buf.clear();
        stdin.read_line(&mut buf)?;
        let (cmd, msg) = buf.split_once(':').unwrap_or(("help", ""));
        let msg = msg.trim();
        match cmd {
            "text" | "send" => {
                ws.send(msg).await?;
                println!("[ECHO] Text: {}", ws::read_msg!(ws)?);
            }
            "ping" => ws.send(Event::Ping(msg.as_bytes())).await?,
            "pong" => ws.send(Event::Pong(msg.as_bytes())).await?,
            "q" | "quit" | "exit" => break msg,
            _ => {
                println!("{USAGE}");
                continue;
            }
        }
    };
    ws.close((CloseCode::Normal, msg)).await
}

fn on_event(ev: Event, pre: &str) {
    match ev {
        Event::Ping(_) => println!("{pre} Ping: {ev}"),
        Event::Pong(_) => println!("{pre} Pong: {ev}"),
    }
}

fn main() {
    let mut args = std::env::args();
    match args.nth(1).as_deref() {
        Some("server" | "-s") => {
            let host = args.next().unwrap_or("0.0.0.0".into());
            let port = if host.contains(':') { "" } else { ":80" };
            utils::block_on(server(format!("{host}{port}"))).unwrap();
        }
        Some("client" | "-c") => {
            let uri = args.next().unwrap_or("localhost:80".into());
            utils::block_on(client(uri)).unwrap();
        }
        _ => println!("{HELP}"),
    }
}
