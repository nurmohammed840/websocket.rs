use super::*;

impl Websocket<CLIENT> {
    pub async fn connect(addr: impl ToSocketAddrs, request: impl AsRef<str>) -> Result<Self> {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(request.as_ref().as_bytes()).await?;

        let mut stream = BufReader::new(stream);
        let _data = stream.fill_buf().await?;

        // use handshake::GetSecKey;
        // let http_req = std::str::from_utf8(data)
        //     .map_err(|error| Error::new(ErrorKind::InvalidData, error))?
        //     .strip_prefix("HTTP/1.1 101 Switching Protocols\r\n")
        //     .ok_or(Error::new(ErrorKind::InvalidData, "error"))?;

        // let headers = handshake::http_headers_from_raw(http_req);

        // let _a = headers
        //     .get_sec_accept_key()
        //     .ok_or(Error::new(ErrorKind::InvalidData, "error"))?;

        // handshake::sec_accept_key_from(sec_key)

        Ok(Self {
            stream,
            len: 0,
            fin: true,
        })
    }

    pub async fn recv<'a>(&'a mut self) -> Result<Data> {
        Ok(client::Data {
            ty: self.read_data_frame_header().await?,
            ws: self,
        })
    }
}

pub struct Data<'a> {
    pub ty: DataType,
    pub(crate) ws: &'a mut Websocket<CLIENT>,
}

default_impl_for_data!();

impl<'a> Data<'a> {
    async fn _next_frag(&mut self) -> Result<()> {
        self.ws.read_fragmented_header().await
    }

    #[inline]
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amt = read_bytes(
            &mut self.ws.stream,
            buf.len().min(self.ws.len),
            |bytes| unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), bytes.len());
            },
        )
        .await?;
        self.ws.len -= amt;
        Ok(amt)
    }
}