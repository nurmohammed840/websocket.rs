use std::str;
use tokio::io::*;
use web_socket::*;

pub async fn echo<IO>(mut ws: WebSocket<IO>) -> Result<()>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = Vec::<u8>::with_capacity(4096);
    loop {
        match ws.recv_frame().await? {
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
            Event::Error(_) => return ws.close(CloseCode::ProtocolError).await,
            Event::Close { .. } => return ws.close(()).await,
        }
    }
}

async fn send_msg<IO>(ws: &mut WebSocket<IO>, ty: MessageType, buf: &[u8]) -> Result<()>
where
    IO: Unpin + AsyncWrite,
{
    match ty {
        MessageType::Text => {
            let msg =
                str::from_utf8(buf).map_err(|error| Error::new(ErrorKind::InvalidData, error))?;
            ws.send(msg).await
        }
        MessageType::Binary => ws.send(buf).await,
    }
}
