mod answer;
mod utils;

use tokio::{io::*, net::TcpListener};
use utils::Http;
use web_socket::*;

use crate::utils::handshake;

#[allow(dead_code)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Some(mut addr) = std::env::args().nth(1)  else {
        return Ok(println!("Try: cargo run --example echo -- 127.0.0.1:8080"));
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

        let Some(key) = get_sec_key(&http) else {
            panic!("[{addr}] error: expected websocket upgrade request");
        };

        stream
            .write_all(handshake::response(key, [("x-agent", "web-socket")]).as_bytes())
            .await?;

        tokio::spawn(async {
            let _ = answer::echo(WebSocket::server(stream)).await;
        });
    }
}

fn get_sec_key(http: &Http) -> Option<&String> {
    if !http.get("connection")?.eq_ignore_ascii_case("upgrade")
        || !http.get("upgrade")?.eq_ignore_ascii_case("websocket")
    {
        return None;
    }
    http.get("sec-websocket-key")
}
