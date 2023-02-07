use super::*;

impl<Stream> WebSocket<SERVER, Stream> {
    #[inline]
    pub fn new(stream: Stream) -> Self {
        Self::from(stream)
    }
}

impl<RW: Unpin + AsyncBufRead + AsyncWrite> WebSocket<SERVER, RW> {
    pub async fn recv(&mut self) -> Result<Data<RW>> {
        let (ty, mask) = cls_if_err!(self, {
            let ty = self.read_data_frame_header().await?;
            let mask = Mask::from(read_buf(&mut self.stream).await?);
            Result::<_>::Ok((ty, mask))
        })?;
        Ok(server::Data { ty, mask, ws: self })
    }
}

pub struct Data<'a, Stream> {
    pub ty: DataType,
    pub(crate) mask: Mask,

    pub(crate) ws: &'a mut WebSocket<SERVER, Stream>,
}

impl<RW: Unpin + AsyncBufRead + AsyncWrite> Data<'_, RW> {
    async fn _read_next_frag(&mut self) -> Result<()> {
        self.ws.read_fragmented_header().await?;
        self.mask = Mask::from(read_buf(&mut self.ws.stream).await?);
        Ok(())
    }

    async fn _read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amt = read_bytes(&mut self.ws.stream, buf.len().min(self.ws.len), |bytes| {
            bytes
                .iter()
                .zip(&mut self.mask)
                .zip(buf.iter_mut())
                .for_each(|((byte, key), dist)| *dist = byte ^ key);
        })
        .await?;
        self.ws.len -= amt;
        Ok(amt)
    }
}

default_impl_for_data!();
