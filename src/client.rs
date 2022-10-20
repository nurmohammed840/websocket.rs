use super::*;

pub struct Data<'a> {
    pub ty: DataType,
    pub(crate) ws: &'a mut Websocket<CLIENT>,
}

default_impl_for_data!();

impl<'a> Data<'a> {
    #[inline]
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = read_bytes(
            &mut self.ws.stream,
            buf.len().min(self.ws.len),
            |bytes| unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), bytes.len());
            },
        )
        .await?;

        self.ws.len -= amt;
        if !self.ws.fin && self.ws.len == 0 {
            self.ws.next_fragmented_header().await?;
        }
        Ok(amt)
    }
}