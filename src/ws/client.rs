use super::*;
use http::{HeaderField, SecWebSocketKey};
use std::net::SocketAddr;

fn parse_ws_uri(uri: &str) -> std::result::Result<(bool, &str, &str), &'static str> {
    let err = "Invalid Websocket URI";
    let uri = uri.strip_prefix("ws").ok_or(err)?;
    let (secure, uri) = match uri.strip_prefix("s") {
        Some(uri) => (true, uri),
        None => (false, uri),
    };
    let uri = uri.strip_prefix("://").ok_or(err)?;
    let (addr, path) = uri.split_once("/").unwrap_or((uri, ""));
    Ok((secure, addr, path))
}

impl Websocket<CLIENT> {
    pub async fn connect(uri: impl AsRef<str>) -> Result<Self> {
        Self::connect_with_headers(uri, [("", ""); 0]).await
    }

    pub async fn connect_with_headers(
        uri: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl HeaderField>,
    ) -> Result<Self> {
        let (secure, addr, path) = parse_ws_uri(uri.as_ref()).map_err(invalid_input)?;
        let default_port = addr.contains(":").then_some("").unwrap_or(match secure {
            true => ":443",
            false => ":80",
        });

        let addrs: Box<[SocketAddr]> = lookup_host(format!("{addr}{default_port}"))
            .await?
            .collect();

        let mut stream = BufReader::new(TcpStream::connect(&*addrs).await?);

        let (request, sec_key) = handshake::request(addr, path, headers);
        stream.get_mut().write_all(request.as_bytes()).await?;

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

        let amt = data.len();
        stream.consume(amt);

        Ok(Self {
            stream,
            len: 0,
            fin: true,
            event: Box::new(|_| Ok(())),
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

impl Data<'_> {
    async fn _next_frag(&mut self) -> Result<()> {
        self.ws.read_fragmented_header().await
    }

    #[inline]
    async fn _read(&mut self, buf: &mut [u8]) -> Result<usize> {
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

default_impl_for_data!();
