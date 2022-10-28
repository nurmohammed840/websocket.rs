mod utils;

use std::{io::Result, str};
use tokio::{io::AsyncReadExt, net::TcpListener, spawn};
use utils::upgrade_websocket;
use web_socket::{CloseCode, WebSocket};

fn main() -> Result<()> {
    utils::block_on(async {
        let server = spawn(server());
        let client = spawn(client());
        let _ = server.await?; // ignore close event
        client.await?
    })
}

macro_rules! read_msg {
    ($ws:expr) => {{
        let mut data = $ws.recv().await?;
        let mut msg = vec![];
        data.read_to_end(&mut msg).await?;
        String::from_utf8(msg).unwrap()
    }};
}

async fn server() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1234").await?;
    println!("[Server] Listening at {}", listener.local_addr()?);

    let (mut stream, addr) = listener.accept().await?;
    println!("[Server] Incoming request from {addr}\n");

    let mut buf = [0; 8096];
    let amt = stream.read(&mut buf).await?;
    println!("{}", str::from_utf8(&buf[..amt]).unwrap());

    let mut ws = upgrade_websocket(&buf[..amt], stream).await?;
    ws.send("Hello, World!").await?;
    loop {
        println!("Client: {}", read_msg!(ws));
    }
}

async fn client() -> Result<()> {
    let mut ws = WebSocket::connect("ws://localhost:1234/chat").await?;
    println!("[Client] Connected to {}", ws.stream.get_ref().peer_addr()?);

    println!("Server: {}", read_msg!(ws));
    ws.send("Hi there!").await?;
    ws.send("Bye!").await?;
    ws.close(CloseCode::Normal, "").await
}
