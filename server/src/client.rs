use super::*;

pub struct Data<'a> {
    pub(crate) fin: bool,
    pub(crate) len: usize,
    pub(crate) ty: DataType,

    pub(crate) ws: &'a mut Websocket<CLIENT>,
}

default_impl_for_data!();

impl<'a> Data<'a> {
    #[inline]
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = read_bytes(
            &mut self.ws.stream,
            buf.len().min(self.len),
            |bytes| unsafe {
                let count = bytes.len();
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), count);
                count
            },
        )
        .await?;
        self.len -= amt;
        if !self.fin && self.len == 0 {
            let (fin, opcode, len) = self.ws.header().await?;
            if opcode != 0 {
                return err("Expected fragment frame");
            }
            self.fin = fin;
            self.len = len;
        }
        Ok(amt)
    }
}
