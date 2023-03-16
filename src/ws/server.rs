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

def_ws!(SERVER, footer);