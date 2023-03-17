#![allow(warnings)]

mod utils;

use std::{io::Result, str};
use tokio::net::{TcpListener, TcpStream};
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
        _ => println!("{HELP}")
    }
}

mod server {
    use super::*;
    use hyper::{
        body::Incoming, header::*, http::HeaderValue, server::conn::http1, service::service_fn,
        upgrade, Request, Response, StatusCode,
    };

    const ADDR: &str = "127.0.0.1:9002";

    pub async fn main() {
        let listener = TcpListener::bind(ADDR).await.unwrap();
        println!("Listening on: {ADDR}");

        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(stream, service_fn(server_upgrade))
                    .await
                {
                    println!("[{addr}] Error serving connection: {err:#?}");
                }
            });
        }
    }

    async fn server_upgrade(mut req: Request<Incoming>) -> hyper::Result<Response<String>> {
        let headers = req.headers();
        let mut responce = Response::builder();

        let sec_ws_key = headers.get(SEC_WEBSOCKET_KEY);
        if !headers.contains_key(UPGRADE)
            || !headers.contains_key(CONNECTION)
            || sec_ws_key.is_none()
        {
            let response = responce
                .status(StatusCode::BAD_REQUEST)
                .body(String::from("expected websocket upgrade request"))
                .unwrap();

            return Ok(response);
        }
        let ws_accept = utils::handshake::accept_key_from(sec_ws_key.unwrap());

        tokio::spawn(async move {
            match upgrade::on(&mut req).await {
                Ok(stream) => {
                    if let Err(err) = handle(WebSocket::server(stream)).await {
                        eprintln!("ws error: {err:#?}")
                    }
                }
                Err(err) => eprintln!("upgrade error: {err:#?}"),
            }
        });
        let responce = responce
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "Upgrade")
            .header(SEC_WEBSOCKET_ACCEPT, ws_accept)
            .body(String::new())
            .unwrap();

        Ok(responce)
    }
}

mod client {
    use super::*;
    use hyper::client::conn::http1;
    use tokio::net::TcpStream;

    const ADDR: &str = "localhost:9001";
    const AGENT: &str = "agent=web-socket";

    async fn connect(
        addr: &str,
        path: &str,
    ) -> Result<WebSocket<CLIENT, hyper::upgrade::Upgraded>> {
        let stream = TcpStream::connect(addr).await?;
        // let (mut sender, conn) = http1::handshake(stream).await.unwrap();
        todo!()
    }

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

async fn handle<const SIDE: bool>(mut ws: WebSocket<SIDE, hyper::upgrade::Upgraded>) -> Result<()> {
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
