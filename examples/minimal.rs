mod utils;
use std::io::Result;
use web_socket::{Event, CloseCode, DataType, MessageType};

#[tokio::main]
async fn main() -> Result<()> {
    let mut ws = utils::connect("ws.ifelse.io:80", "/").await?;
    for _ in 0..3 {
        ws.send("Copy Cat!").await?;

        match ws.recv().await? {
            Event::Data { ty, data } => {
                assert_eq!(ty, DataType::Complete(MessageType::Text));
                assert_eq!(&data[..], b"Copy Cat!");
            },
            Event::Ping(data) => ws.send_ping(data).await?,
            Event::Pong(..) => {},
            Event::Error(..) => return ws.close(CloseCode::ProtocolError).await,
            Event::Close { .. } => return ws.close(()).await
        }
    }
    ws.close("bye!").await
}
