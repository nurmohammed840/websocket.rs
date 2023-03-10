use super::*;

impl<IO> WebSocket<SERVER, IO> {
    /// Create a websocket server instance.
    #[inline]
    pub fn server(stream: IO) -> Self {
        Self {
            stream,
            _is_closed: false,
            done: true,
        }
    }
}

impl<IO: Unpin + AsyncRead> WebSocket<SERVER, IO> {
    /// reads [Data] from websocket stream.
    #[inline]
    pub async fn recv(&mut self) -> Result<Event> {
        let result = if self.done {
            self._recv().await
        } else {
            self.next().await
        }?;
        match result {
            Either::Data((ty, done, len)) => {
                // match (self.done, ty) {
                //     (true, DataType::Continue) => return Ok(Event::Error("expected data frame")),
                //     (false, DataType::Text | DataType::Binary) => {
                //         return Ok(Event::Error("expected fragment frame"))
                //     }
                //     _ => self.done = done
                // }
                let keys: [u8; 4] = read_buf(&mut self.stream).await?;

                let mut data = vec![0; len].into_boxed_slice();
                self.stream.read_exact(&mut data).await?;

                for (i, byte) in data.iter_mut().enumerate() {
                    *byte ^= keys[i % 4];
                }
                Ok(Event::Data { ty, done, data })
            }
            Either::Event(ev) => Ok(ev),
        }
    }
}