use super::*;

impl<IO> WebSocket<CLIENT, IO> {
    /// Create a new websocket client instance.
    pub fn client(stream: IO) -> Self {
        Self::from(stream)
    }
}

#[inline]
pub async fn footer<const SIDE: bool, R>(
    this: &mut WebSocket<SIDE, R>,
    ty: DataType,
    len: usize,
) -> Result<Event>
where
    R: Unpin + AsyncRead,
{
    let mut data = vec![0; len].into_boxed_slice();
    this.stream.read_exact(&mut data).await?;
    Ok(Event::Data { ty, data })
}
