use axum::{extract::ConnectInfo, response::IntoResponse, routing::get, Router};
use axum_example::{WebSocket, WebSocketUpgrade};
use std::{io, net::SocketAddr};
use tokio::net::TcpListener;
use web_socket::Event;

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = Router::new().route("/ws", get(ws_handler));
    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    println!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

async fn ws_handler(ws: WebSocketUpgrade, info: ConnectInfo<SocketAddr>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, info.0))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    println!("From: {:#?}", who);
    let _ = socket.send("Hello Client!").await;

    while let Ok(ev) = socket.recv().await {
        match ev {
            Event::Data { ty, data } => {
                println!("DataType: {:?}", ty);
                println!("Data: {:?}", String::from_utf8(data.to_vec()));
            }
            Event::Ping(_) => {}
            Event::Pong(_) => {}
            Event::Error(_) => {}
            Event::Close { .. } => {}
        }
    }
}
