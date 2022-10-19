use super::*;

pub struct Data<'a> {
    pub(crate) ty: DataType,
    pub(crate) ws: &'a mut Websocket<CLIENT>,
}

default_impl_for_data!();

impl<'a> Data<'a> {
    #[inline]
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len().min(self.ws.len);
        let buf = unsafe { buf.get_mut(..len).unwrap_unchecked() };
        let amt = self.ws.stream.read(buf).await?;

        self.ws.len -= amt;
        if !self.ws.fin && self.ws.len == 0 {
            self.ws.read_fragmented_header().await?;
        }
        Ok(amt)
    }
}
