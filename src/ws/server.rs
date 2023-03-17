use super::*;

impl<IO> WebSocket<SERVER, IO> {
    /// Create a websocket server instance.
    #[inline]
    pub fn server(stream: IO) -> Self {
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
    let mask: [u8; 4] = read_buf(&mut this.stream).await?;

    let mut data = vec![0; len].into_boxed_slice();
    this.stream.read_exact(&mut data).await?;
    utils::apply_mask(&mut data, mask);

    Ok(Event::Data { ty, data })
}
