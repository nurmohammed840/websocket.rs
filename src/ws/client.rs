use super::*;
use http::{HeaderField, SecWebSocketKey};

impl Websocket<CLIENT> {
    pub async fn connect(addr: impl ToSocketAddrs + std::fmt::Display) -> Result<Self> {
        Self::connect_with_headers(addr, [("", ""); 0]).await
    }

    pub async fn connect_with_headers(
        addr: impl ToSocketAddrs + std::fmt::Display,
        headers: impl IntoIterator<Item = impl HeaderField>,
    ) -> Result<Self> {
        let host = format!("{addr}");
        let mut stream = TcpStream::connect(addr).await?;

        let (request, sec_key) = handshake::request(host, "/", headers);
        stream.write_all(request.as_bytes()).await?;

        let mut stream = BufReader::new(stream);

        let data = stream.fill_buf().await?;
        let responce = std::str::from_utf8(data)
            .map_err(invalid_data)?
            .strip_prefix("HTTP/1.1 101 Switching Protocols\r\n")
            .ok_or(invalid_data("Invalid HTTP response"))?;

        let headers = http::headers_from_raw(responce);
        let accept_key = headers
            .get_sec_ws_accept_key()
            .ok_or(invalid_data("Couldn't get `Accept-Key` from response"))?;

        if handshake::accept_key_from(sec_key) != accept_key {
            return Err(invalid_data("Invalid accept key"));
        }

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

impl Data<'_> {
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_name() -> Result<()> {
        let mut ws = Websocket::connect("127.0.0.1:8080").await?;
        ws.send("msg").await?;

        let mut data = ws.recv().await?;

        let mut buf = vec![];
        data.read_to_end(&mut buf).await?;
        println!("{:?}", buf);
        Ok(())
    }

    #[tokio::test]
    async fn test() {
        println!("{:?}", test_name().await);
    }
}
