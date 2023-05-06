use std::str;
use tokio::io::*;
use web_socket::*;

pub async fn echo<IO>(mut ws: WebSocket<IO>) -> Result<()>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = Vec::with_capacity(4096);
    let mut frag_msg: Option<MessageType> = None;
    loop {
        match ws.recv().await? {
            Event::Data { ty, data } => match ty {
                DataType::Fragment(chunk) => {
                    match chunk {
                        Fragment::Start(ty) if frag_msg.replace(ty).is_some() => return Ok(()),
                        Fragment::Next if frag_msg.is_none() => return Ok(()),
                        _ => buf.extend_from_slice(&data),
                    }
                    if let Fragment::End = chunk {
                        let Some(ty) = frag_msg.take() else { return Ok(()) };
                        send_msg(&mut ws, ty, &buf).await?;
                        buf.clear();
                    }
                }
                DataType::Complete(ty) => {
                    if frag_msg.is_some() {
                        // expected fragment, but got data
                        return Ok(());
                    }
                    send_msg(&mut ws, ty, &data).await?;
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
