mod utils;
mod answer;

use tokio::{io::*, net::TcpListener};
use utils::Http;
use web_socket::*;

use crate::utils::handshake;

#[allow(dead_code)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Some(mut addr) = std::env::args().nth(1)  else {
        return Ok(println!(""));
    };
    if !addr.contains(":") {
        addr.push_str(":8080");
    }
    let listener = TcpListener::bind(&addr).await?;
    println!("[Server] Listening at {addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut stream = BufReader::new(stream);
        let http = Http::parse(&mut stream).await?;

        let key = http.get("sec-websocket-key");
        if !utils::contains(&http.get("connection"), "Upgrade")
            || !utils::contains(&http.get("upgrade"), "websocket")
            || key.is_none()
        {
            panic!("[{addr}] error: expected websocket upgrade request");
        }

        stream
            .write_all(handshake::response(key.unwrap(), [("x-agent", "web-socket")]).as_bytes())
            .await?;

        tokio::spawn(async {
            if let Err(err) = answer::echo(WebSocket::server(stream)).await {
                println!("ws error: {err:#?}")
            }
        });
    }
}

