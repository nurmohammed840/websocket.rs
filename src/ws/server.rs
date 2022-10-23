use super::*;

impl Websocket<SERVER> {
    pub fn new(stream: BufReader<TcpStream>) -> Self {
        Self {
            stream,
            len: 0,
            fin: true,
        }
    }

    pub async fn recv<'a>(&'a mut self) -> Result<Data> {
        let ty = self.read_data_frame_header().await?;
        let mask = Mask::from(read_buf(&mut self.stream).await?);
        Ok(server::Data { ty, mask, ws: self })
    }
}

pub struct Data<'a> {
    pub ty: DataType,
    pub(crate) mask: Mask,

    pub(crate) ws: &'a mut Websocket<SERVER>,
}

default_impl_for_data!();

impl Data<'_> {
    async fn _next_frag(&mut self) -> Result<()> {
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
