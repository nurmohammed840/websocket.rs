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
        let len = buf.len().min(self.len);
        let buf = unsafe { buf.get_mut(..len).unwrap_unchecked() };
        let amt = self.ws.stream.read(buf).await?;
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
