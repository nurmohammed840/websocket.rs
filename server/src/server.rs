use super::*;

pub struct Data<'a> {
    pub(crate) ty: DataType,
    pub(crate) mask: Mask,

    pub(crate) ws: &'a mut Websocket<SERVER>,
}

default_impl_for_data!();

#[inline]
pub async fn read_bytes(
    stream: &mut BufReader<TcpStream>,
    len: usize,
    cb: impl FnOnce(&[u8]) -> usize,
) -> io::Result<usize> {
    let bytes = stream.fill_buf().await?;
    let amt = bytes.len().min(len);
    let count = cb(unsafe { bytes.get_unchecked(..amt) });
    stream.consume(amt);
    Ok(count)
}

impl<'a> Data<'a> {
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = read_bytes(&mut self.ws.stream, buf.len().min(self.ws.len), |bytes| {
            bytes
                .iter()
                .zip(&mut self.mask)
                .zip(buf.iter_mut())
                .for_each(|((byte, key), dist)| *dist = byte ^ key);

            bytes.len()
        })
        .await?;
        self.ws.len -= amt;
        if !self.ws.fin && self.ws.len == 0 {
            self.ws.read_fragmented_header().await?;
            self.mask = Mask::from(read_buf(&mut self.ws.stream).await?);
        }
        Ok(amt)
    }
}
