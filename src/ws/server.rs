use super::*;

impl<IO> WebSocket<SERVER, IO> {
    /// Create a websocket server instance.
    #[inline]
    pub fn server(stream: IO) -> Self {
        Self::from(stream)
    }
}

#[inline]
async fn footer<IO>(this: &mut WebSocket<SERVER, IO>, ty: DataType, len: usize) -> Result<Event>
where
    IO: Unpin + AsyncRead,
{
    let mask: [u8; 4] = read_buf(&mut this.stream).await?;

    let mut data = vec![0; len].into_boxed_slice();
    this.stream.read_exact(&mut data).await?;
    utils::apply_mask(&mut data, mask);

    Ok(Event::Data {
        ty,
        done: this.done,
        data,
    })
}

impl<IO: Unpin + AsyncRead> WebSocket<SERVER, IO> {
    /// reads [Event] from websocket stream.
    #[inline]
    pub async fn recv(&mut self) -> Result<Event> {
        if self.is_closed {
            io_err!(NotConnected, "read after close");
        }
        let event = self.header(footer).await;
        if let Ok(Event::Close { .. } | Event::Error(..)) | Err(..) = event {
            self.is_closed = true;
        }
        event
    }
}
