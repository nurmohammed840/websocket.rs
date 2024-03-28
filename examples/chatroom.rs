//! A chat example using actor pattern.
//!
//! An actor is nothing but some kind of concurrent (task, thread, process, ...) operation, which communicates with the outside world using channels.
//! For example: javascript web-worker.
//!
//! Run: cargo r -r --example chatroom -- 127.0.0.1:8080

mod utils;

use std::{collections::HashMap, error::Error, net::SocketAddr, str, sync::Arc};
use tokio::{io::*, net::TcpListener, select, sync::mpsc};
use utils::{handshake, HttpRequest};
use web_socket::*;

type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;
type Sender<T> = mpsc::UnboundedSender<T>;

#[tokio::main]
async fn main() -> Result {
    let addr = std::env::args().nth(1).unwrap_or("127.0.0.1:8080".into());
    let listener = TcpListener::bind(&addr).await?;
    println!("[Server] Listening at {addr}");
    println!("Goto: http://localhost:8080/");

    let mut room: HashMap<SocketAddr, Sender<Message>> = HashMap::new();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Command>();

    // Because both `tokio::sync::mpsc::Receiver::recv` and `tokio::net::TcpListener::accept` are cancellation safe. Its ok to use `select!` in main loop.
    // Otherwise you may want to move any part into separate task. For example:

    // tokio::spawn(async move {
    //     while let Some(cmd) = cmd_rx.recv().await {
    //         match cmd {
    //             Command::JoinRoom { .. } => { .. },
    //             Command::RemoveUser { .. } => { .. },
    //             ...
    //         }
    //     }
    // });
    // loop {
    //     while let Ok((stream, addr)) = listener.accept().await {
    //         // ...
    //     }
    // }

    // Basic HTTP server
    loop {
        let cmd_tx = cmd_tx.clone();
        select! {
            Ok((stream, addr)) = listener.accept() => {
                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);

                let req = HttpRequest::parse(&mut reader).await?;
                if req.prefix.starts_with("GET / HTTP/1.1") {
                    let content = include_str!("./assets/chatroom.html");
                    let content_len = content.len();
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {content_len}\r\n\r\n{content}");
                    writer.write_all(response.as_bytes()).await?;
                }
                if let Some(key) = utils::get_sec_key(&req) {
                    let res = handshake::response(key, [("x-agent", "web-socket")]);
                    println!("From: {addr}\n{req:#?}");
                    println!("\nTo: {addr}\n{res}");
                    writer.write_all(res.as_bytes()).await?;

                    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
                    room.insert(addr, tx.clone());

                    let mut ws = WebSocket::server(writer);
                    tokio::spawn(async move {
                        let _ = handle_client(cmd_tx, addr, tx, WebSocket::server(reader)).await;
                    });
                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            let _ = match msg {
                                Message::Ping(data) => ws.send_pong(data).await,
                                Message::Reply(msg) => ws.send(&*msg).await,
                            };
                        }
                    });
                }
            }
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Command::JoinRoom { user_addr, user_tx } => {
                        if room.get(&user_addr).is_none() {
                            room.insert(user_addr, user_tx);
                            for user_tx in room.values() {
                                let _ = user_tx.send(Message::Reply(format!("New User: {user_addr}").into()));
                            }
                        }
                    },
                    Command::RemoveUser { user_addr } => {
                        room.remove(&user_addr);
                        for user_tx in room.values() {
                            let _ = user_tx.send(Message::Reply(
                                format!("User: {user_addr} Disconnected").into(),
                            ));
                        }
                    },
                    Command::Send(msg) => {
                        for user_tx in room.values() {
                            let _ = user_tx.send(Message::Reply(msg.clone()));
                        }
                    },

                }
            }
        };
    }
}

enum Command {
    JoinRoom {
        user_addr: SocketAddr,
        user_tx: Sender<Message>,
    },
    RemoveUser {
        user_addr: SocketAddr,
    },
    // `Arc<str>` is cheap to clone
    Send(Arc<str>),
}

enum Message {
    Reply(Arc<str>),
    Ping(Box<[u8]>),
}

async fn handle_client<R>(
    cmd: Sender<Command>,
    addr: SocketAddr,
    tx: Sender<Message>,
    mut ws: WebSocket<R>,
) -> Result
where
    R: AsyncRead + Send + Unpin + 'static,
{
    cmd.send(Command::Send(format!("New User: {addr}").into()))?;
    while let Ok(ev) = ws.recv().await {
        match ev {
            Event::Data { data, .. } => {
                let (kind, data) = str::from_utf8(&data)?.split_once(":").unwrap_or_default();
                match kind {
                    "Echo" => tx.send(Message::Reply(data.into()))?,

                    "Send" => cmd.send(Command::Send(
                        format!("From: {addr}, Message: {data}").into(),
                    ))?,

                    "JoinRoom" => cmd.send(Command::JoinRoom {
                        user_tx: tx.clone(),
                        user_addr: addr,
                    })?,

                    "NOOP" => {}
                    _unknown_command => {}
                }
            }
            Event::Ping(msg) => tx.send(Message::Ping(msg))?,
            Event::Pong(_) => {}
            Event::Error(_) | Event::Close { .. } => break,
        }
    }
    cmd.send(Command::RemoveUser { user_addr: addr })?;
    Ok(())
}
