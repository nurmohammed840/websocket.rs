use tokio::io::AsyncBufReadExt;
use super::*;

pub struct Data<'a> {
    pub(crate) fin: bool,
    pub(crate) len: usize,
    pub(crate) ty: DataType,

    pub(crate) ws: &'a mut Websocket<CLIENT>,
}

impl Data<'_> {
    pub fn ty(&self) -> DataType {
        self.ty
    }
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a> Data<'a> {
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.len == 0 {
            if self.fin {
                return Ok(0);
            }
            let (fin, opcode, len) = self.ws.header().await?;
            if opcode != 0 || len == 0 {
                return err("Expected fragment, But got data, And length shouldn't be zero");
            }
            self.fin = fin;
            self.len = len;
        }
        let amt = read_bytes(&mut self.ws.stream, buf.len().min(self.len), |_bytes| {}).await?;
        self.len -= amt;
        Ok(amt)
    }

    pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        utils::default_read_exact!(self, buf)
    }
}

#[inline]
async fn read_bytes<F>(stream: &mut BufReader<TcpStream>, len: usize, cb: F) -> io::Result<usize>
where
    F: FnOnce(&[u8]),
{
    let bytes = stream.fill_buf().await?;
    let amt = bytes.len().min(len);
    cb(unsafe { bytes.get_unchecked(..amt) });
    stream.consume(amt);
    Ok(amt)
}