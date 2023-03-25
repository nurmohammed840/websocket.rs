use std::str;
use tokio::io::*;
use web_socket::*;

pub async fn echo<const SIDE: bool, IO>(mut ws: WebSocket<SIDE, IO>) -> Result<()>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
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
