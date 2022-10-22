use super::*;

macro_rules! default_impl_for_data {
    () => {
        impl Data<'_> {
            #[inline]
            pub fn len(&self) -> usize {
                self.ws.len
            }

            #[inline]
            pub fn fin(&self) -> bool {
                self.ws.fin
            }

            #[inline]
            pub async fn send(&mut self, data: impl Frame) -> Result<()> {
                self.ws.send(data).await
            }

            #[inline]
            pub async fn recv_next(&mut self) -> Result<bool> {
                if self.ws.len > 0 {
                    return Ok(true);
                }
                match self.ws.fin {
                    true => Ok(false),
                    false => {
                        self._next_frag().await?;
                        Ok(true)
                    }
                }
            }

            #[inline]
            pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
                while !buf.is_empty() {
                    match self.read(buf).await? {
                        0 => break,
                        amt => buf = &mut buf[amt..],
                    }
                }
                match buf.is_empty() {
                    true => Ok(()),
                    false => Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    )),
                }
            }

            #[inline]
            pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
                let len = buf.len();
                let additional = self.ws.len;
                buf.reserve(additional);

                unsafe {
                    let end = buf.as_mut_ptr().add(len);
                    let mut uninit = std::slice::from_raw_parts_mut(end, additional);
                    
                    self.read_exact(&mut uninit).await?;
                    buf.set_len(len + additional);
                }
                Ok(additional)
            }
        }
    };
}

pub(crate) use default_impl_for_data;

pub async fn read_buf<const N: usize>(stream: &mut BufReader<TcpStream>) -> Result<[u8; N]> {
    let mut buf = [0; N];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

#[inline]
pub async fn read_bytes(
    stream: &mut BufReader<TcpStream>,
    len: usize,
    cb: impl FnOnce(&[u8]),
) -> Result<usize> {
    let bytes = stream.fill_buf().await?;
    if bytes.is_empty() {
        return conn_aborted();
    }
    let amt = bytes.len().min(len);
    cb(unsafe { bytes.get_unchecked(..amt) });
    stream.consume(amt);
    Ok(amt)
}
