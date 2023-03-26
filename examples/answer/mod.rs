use std::str;
use tokio::io::*;
use web_socket::*;

pub async fn echo<const SIDE: bool, IO>(mut ws: WebSocket<SIDE, IO>) -> Result<()>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = Vec::with_capacity(4096);
    let mut frag_ty: Option<MessageType> = None;
    loop {
        match ws.recv().await? {
            Event::Data { ty, data } => match ty {
                DataType::Fragment(chunk) => match chunk {
                    Fragment::Start(ty) => {
                        if frag_ty.replace(ty).is_some() {
                            return Ok(());
                        }
                        buf.extend_from_slice(&data);
                    }
                    Fragment::Next => {
                        if frag_ty.is_none() {
                            return Ok(());
                        }
                        buf.extend_from_slice(&data);
                    }
                    Fragment::End => {
                        let Some(ty) = frag_ty.take() else { return Ok(()) };
                        buf.extend_from_slice(&data);
                        match ty {
                            MessageType::Text => match str::from_utf8(&buf) {
                                Ok(msg) => ws.send(msg).await?,
                                Err(_) => return Ok(()),
                            },
                            MessageType::Binary => ws.send(&*buf).await?,
                        }
                        buf.clear();
                    }
                },
                DataType::Complete(ty) => {
                    if frag_ty.is_some() {
                        // expected fragment, but got data
                        return Ok(());
                    }
                    match ty {
                        MessageType::Text => match str::from_utf8(&data) {
                            Ok(msg) => ws.send(msg).await?,
                            Err(_) => return Ok(()),
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
