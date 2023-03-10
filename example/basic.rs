mod utils;

use std::io::Result;
use tokio::{net::TcpListener, spawn};
use utils::ws;
use web_socket::client::WS;

fn main() -> Result<()> {
    utils::block_on(async {
        let server = spawn(server());
        let client = spawn(client());
        let _ = server.await?; // ignore event
        client.await?
    })
}

async fn server() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1234").await?;
    println!("[Server] Listening at {}", listener.local_addr()?);

    let (stream, _addr) = listener.accept().await?;

    let mut ws = ws::upgrade(stream).await?;
    ws.send("Hello, World!").await?;
    loop {
        println!("Client: {}", read_msg!(ws)?);
    }
}

async fn client() -> Result<()> {
    let mut ws = WS::connect("localhost:1234", "/chat").await?;
    println!("[Client] Connected to {}", ws.stream.get_ref().peer_addr()?);

    println!("Server: {}", read_msg!(ws)?);
    ws.send("Hi there!").await?;
    ws.send("Bye!").await
}
