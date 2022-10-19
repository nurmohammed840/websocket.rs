use super::*;

pub(crate) struct Mask {
    index: usize,
    keys: [u8; 4],
}

impl Mask {
    pub fn new(keys: [u8; 4]) -> Self {
        Self { index: 0, keys }
    }
}

impl Iterator for Mask {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys[self.index % 4];
        self.index += 1;
        Some(key)
    }
}

pub struct Data<'a> {
    pub(crate) fin: bool,
    pub(crate) ty: DataType,
    pub(crate) len: usize,
    pub(crate) mask: Mask,

    pub(crate) ws: &'a mut Websocket<SERVER>,
}

default_impl_for_data!();

impl<'a> Data<'a> {
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = read_bytes(&mut self.ws.stream, buf.len().min(self.len), |bytes| {
            bytes
                .iter()
                .zip(&mut self.mask)
                .zip(buf.iter_mut())
                .for_each(|((byte, key), dist)| *dist = byte ^ key);

            bytes.len()
        })
        .await?;
        self.len -= amt;
        if !self.fin && self.len == 0 {
            let (fin, opcode, len) = self.ws.header().await?;
            if opcode != 0 {
                return err("Expected fragment frame");
            }
            self.fin = fin;
            self.len = len;
            self.mask = Mask::new(read_buf(&mut self.ws.stream).await?);
        }
        Ok(amt)
    }
}
