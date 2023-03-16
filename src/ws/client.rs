use super::*;

impl<IO> WebSocket<CLIENT, IO> {
    /// Create a new websocket client instance.
    pub fn client(stream: IO) -> Self {
        utils::set_rand_seed();
        Self::from(stream)
    }
}

#[inline]
async fn footer<IO>(this: &mut WebSocket<CLIENT, IO>, ty: DataType, len: usize) -> Result<Event>
where
    IO: Unpin + AsyncRead,
{
    let mut data = vec![0; len].into_boxed_slice();
    this.stream.read_exact(&mut data).await?;
    Ok(Event::Data { ty, data })
}

def_ws!(CLIENT, footer);
