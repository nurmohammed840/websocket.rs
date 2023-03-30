mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    example(utils::connect("ws.ifelse.io:80", "/").await?).await
}

// ---------------------------------------------------------------------

use tokio::io::*;
use web_socket::*;

async fn example<IO>(mut ws: WebSocket<CLIENT, IO>) -> Result<()>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    let _ = ws.recv().await?; // ignore message: Request served by 4338e324
    for _ in 0..3 {
        ws.send("Copy Cat!").await?;

        match ws.recv().await? {
            Event::Data { ty, data } => {
                assert_eq!(ty, DataType::Complete(MessageType::Text));
                assert_eq!(&*data, b"Copy Cat!");
            }
            Event::Ping(data) => ws.send(Pong(data)).await?,
            Event::Pong(..) => {}
            Event::Error(..) => return ws.close(CloseCode::ProtocolError).await,
            Event::Close { .. } => return ws.close(()).await,
        }
    }
    ws.close("bye!").await
}
