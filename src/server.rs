use super::*;

pub struct Data<'a> {
    pub ty: DataType,
    pub(crate) mask: Mask,

    pub(crate) ws: &'a mut Websocket<SERVER>,
}

default_impl_for_data!();

impl<'a> Data<'a> {
    async fn _next_frag(&mut self) -> io::Result<()> {
        self.ws.read_fragmented_header().await?;
        self.mask = Mask::from(read_buf(&mut self.ws.stream).await?);
        Ok(())
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
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
