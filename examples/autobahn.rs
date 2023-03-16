#![allow(warnings)]

mod utils;

use std::{io::Result, str};
use tokio::net::{TcpListener, TcpStream};
use web_socket::{CloseCode, DataType, Event, Fragment, MessageType, WebSocket, SERVER};

#[tokio::main(flavor = "current_thread")]
async fn main() {}

// ------------------------------------------------------------------------------------

mod server {
    use super::*;
    use hyper::{
        body::Incoming, header::*, http::HeaderValue, server::conn::http1, service::service_fn,
        upgrade, Request, Response, StatusCode,
    };

    const ADDR: &str = "127.0.0.1:9002";

    async fn main() {
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
        let mut res = Response::builder();

        if !req.headers().contains_key(UPGRADE) {
            // res.status(StatusCode::BAD_REQUEST).body(String::from("No upgrade request found."));
            // return res.body("Not".into());
        }

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

        let res = res
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "Upgrade")
            .header(SEC_WEBSOCKET_ACCEPT, "value")
            .body(String::new())
            .unwrap();

        Ok(res)
    }
}

async fn handle(mut ws: WebSocket<SERVER, hyper::upgrade::Upgraded>) -> Result<()> {
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
