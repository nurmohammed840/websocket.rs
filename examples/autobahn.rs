mod utils;

use std::{error::Error, str};
use tokio::io::{AsyncRead, AsyncWrite};
use utils::connect;
use web_socket::*;

type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;

const ADDR: &str = "localhost:9001";
const AGENT: &str = "agent=web-socket";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    let total: u32 = match connect(ADDR, "/getCaseCount").await?.recv().await? {
        Event::Data { data, .. } => str::from_utf8(&data)?.parse()?,
        _ => return Err("unable to get case count".into()),
    };

    for case in 1..=total {
        let path = format!("/runCase?case={case}&{AGENT}");
        let _ = echo(connect(ADDR, &path).await?).await;
    }

    connect(ADDR, &format!("/updateReports?{AGENT}"))
        .await?
        .close(())
        .await?;

    Ok(())
}

pub async fn echo<IO>(mut ws: WebSocket<IO>) -> Result
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = Vec::with_capacity(4096);
    loop {
        match ws.recv_event().await? {
            Event::Data { ty, data } => match ty {
                DataType::Complete(ty) => send_msg(&mut ws, ty, &data).await?,
                DataType::Stream(stream) => {
                    buf.extend_from_slice(&data);
                    if let Stream::End(ty) = stream {
                        send_msg(&mut ws, ty, &buf).await?;
                        buf.clear();
                    }
                }
            },
            Event::Ping(data) => ws.send_pong(data).await?,
            Event::Pong(_) => {}
            Event::Error(_) => break ws.close(CloseCode::ProtocolError).await?,
            Event::Close { .. } => break ws.close(()).await?,
        }
    }
    Ok(())
}

async fn send_msg<IO>(ws: &mut WebSocket<IO>, ty: MessageType, buf: &[u8]) -> Result
where
    IO: Unpin + AsyncWrite,
{
    match ty {
        MessageType::Binary => ws.send(buf).await?,
        MessageType::Text => ws.send(str::from_utf8(buf)?).await?,
    }
    Ok(())
}
