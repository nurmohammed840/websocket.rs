#![allow(warnings)]

mod utils;

use std::{io::Result, str};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use web_socket::*;

const HELP: &str = r#"
USAGE:
    cargo run --example autobahn -- server
    cargo run --example autobahn -- client
"#;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("client" | "-c" | "--client") => client::main().await,
        Some("server" | "-s" | "--server") => server::main().await,
        _ => println!("{HELP}"),
    }
}

mod server {
    use super::*;
    use crate::utils::handshake;
    use httparse::{parse_headers, Status, EMPTY_HEADER};
    use std::collections::HashMap;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use utils::OptionExt;

    const ADDR: &str = "127.0.0.1:9002";

    pub async fn main() {
        let listener = TcpListener::bind(ADDR).await.unwrap();
        println!("Listening on: {ADDR}");

        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(async move {
                let mut stream = BufReader::new(stream);

                // ----------------------- Parse HTTP --------------------

                let data = stream.fill_buf().await.unwrap();
                let mut headers = [EMPTY_HEADER; 16];
                let Ok(Status::Complete((amt, _))) = parse_headers(data, &mut headers) else { return  };
                let headers: HashMap<_, _> = headers.iter().map(|h| (h.name, h.value)).collect();

                // ---------------------- Handshake ---------------------

                let key = headers.get("Sec-WebSocket-Key");
                if !headers.get("Connection").contains(b"Upgrade")
                    || !headers.get("Upgrade").contains(b"websocket")
                    || key.is_none()
                {
                    return println!("[{addr}] error: expected websocket upgrade request");
                }
                let response = handshake::response(key.unwrap(), [("x-server-type", "web-socket")]);
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.consume(amt);

                // --------------------------------------------------------

                if let Err(err) = handle(WebSocket::server(stream)).await {
                    println!("ws error: {err:#?}")
                }
            });
        }
    }
}

mod client {
    use super::*;
    use crate::utils::connect;

    const ADDR: &str = "localhost:9001";
    const AGENT: &str = "agent=web-socket";

    async fn get_case_count() -> Option<u32> {
        let mut ws = connect(ADDR, "/getCaseCount").await.unwrap();
        if let Event::Data { data, .. } = ws.recv().await.unwrap() {
            return std::str::from_utf8(&data).ok()?.parse().ok();
        }
        None
    }

    pub async fn main() {
        let total = get_case_count().await.expect("unable to get case count");
        for case in 1..=total {
            tokio::spawn(async move {
                let mut ws = connect(ADDR, &format!("/runCase?case={case}&{AGENT}"))
                    .await
                    .unwrap();

                if let Err(err) = handle(ws).await {
                    eprintln!("ws error: {err:#?}")
                }
            });
        }
        update_reports().await.expect("unable update reports");
    }

    async fn update_reports() -> Result<()> {
        let ws = connect(ADDR, &format!("/updateReports?{AGENT}")).await?;
        ws.close(()).await
    }
}

async fn handle<const SIDE: bool, R>(mut ws: WebSocket<SIDE, R>) -> Result<()>
where
    R: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = Vec::with_capacity(4096);
    let mut msg_ty = MessageType::Text;
    loop {
        match ws.recv().await? {
            Event::Data { ty, data } => match ty {
                DataType::Fragment(chunk) => match chunk {
                    Fragment::Start(ty) => {
                        msg_ty = ty;
                        buf.extend_from_slice(&data);
                    }
                    Fragment::Next => buf.extend_from_slice(&data),
                    Fragment::End => {
                        buf.extend_from_slice(&data);
                        match msg_ty {
                            MessageType::Text => match str::from_utf8(&buf) {
                                Ok(msg) => ws.send(msg).await?,
                                Err(_) => return ws.close(()).await,
                            },
                            MessageType::Binary => ws.send(&*buf).await?,
                        }
                        buf.clear();
                    }
                },
                DataType::Complete(ty) => {
                    if !buf.is_empty() {
                        // ...
                    }
                    match ty {
                        MessageType::Text => match str::from_utf8(&data) {
                            Ok(msg) => ws.send(msg).await?,
                            Err(_) => return ws.close(()).await,
                        },
                        MessageType::Binary => ws.send(&*data).await?,
                    }
                }
            },
            Event::Ping(data) => ws.send_pong(data).await?,
            Event::Pong(_) => {}
            Event::Error(_) => return ws.close(CloseCode::ProtocolError).await,
            Event::Close { .. } => return ws.close(()).await,
        }
    }
}
