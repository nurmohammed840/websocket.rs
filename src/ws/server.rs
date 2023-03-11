use super::*;

impl<IO> WebSocket<SERVER, IO> {
    /// Create a websocket server instance.
    #[inline]
    pub fn server(stream: IO) -> Self {
        Self::from(stream)
    }
}

impl<IO: Unpin + AsyncRead> WebSocket<SERVER, IO> {
    /// reads [Event] from websocket stream.
    pub async fn recv(&mut self) -> Result<Event> {
        if self.is_closed {
            return Err(Error::new(ErrorKind::NotConnected, "read after close"));
        }
        let result = self
            .header(|this, ty, len| async move {
                let mask: [u8; 4] = read_buf(&mut this.stream).await?;

                let mut data = vec![0; len].into_boxed_slice();
                this.stream.read_exact(&mut data).await?;
                utils::apply_mask(&mut data, mask);

                Ok(Event::Data {
                    ty,
                    done: this.done,
                    data,
                })
            })
            .await;

        if let Ok(Event::Close { .. } | Event::Error(..)) | Err(..) = result {
            self.is_closed = true;
        }
        result
    }
}
