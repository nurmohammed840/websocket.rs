use super::*;

impl<Stream> WebSocket<SERVER, Stream> {
    /// Create a new websocket instance.
    #[inline]
    pub fn new(stream: Stream) -> Self {
        Self::from(stream)
    }
}

impl<IO: Unpin + AsyncRead + AsyncWrite> WebSocket<SERVER, IO> {
    /// reads [Data] from websocket stream.
    #[inline]
    pub async fn recv(&mut self) -> Result<Data<IO>> {
        let (ty, mask) = cls_if_err!(self, {
            let ty = self.read_data_frame_header().await?;
            let mask = Mask::from(read_buf(&mut self.stream).await?);
            Result::<_>::Ok((ty, mask))
        })?;
        Ok(server::Data { ty, mask, ws: self })
    }
}

/// It represent a single websocket message.
pub struct Data<'a, Stream> {
    /// A [DataType] value indicating the type of the data.
    pub ty: DataType,
    pub(crate) mask: Mask,

    pub(crate) ws: &'a mut WebSocket<SERVER, Stream>,
}

impl<IO: Unpin + AsyncRead + AsyncWrite> Data<'_, IO> {
    #[inline]
    async fn _read_next_frag(&mut self) -> Result<()> {
        self.ws.read_fragmented_header().await?;
        self.mask = Mask::from(read_buf(&mut self.ws.stream).await?);
        Ok(())
    }

    #[inline]
    async fn _read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = buf.len().min(self.ws.len);
        if len > 0 {
            let mut bytes = Vec::<u8>::with_capacity(len);
            unsafe {
                let uninit = std::slice::from_raw_parts_mut(bytes.as_mut_ptr(), len);
                len = self.ws.stream.read(uninit).await?;
            }
            bytes[..len]
                .iter()
                .zip(&mut self.mask)
                .zip(buf.iter_mut())
                .for_each(|((byte, key), dist)| *dist = byte ^ key);

            self.ws.len -= len;
        }
        Ok(len)
    }
}

default_impl_for_data!();
