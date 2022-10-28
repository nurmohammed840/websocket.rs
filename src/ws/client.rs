use super::*;
use http::FmtHeaderField;

fn parse_ws_uri(uri: &str) -> std::result::Result<(bool, &str, &str), &'static str> {
    let err_msg = "Invalid Websocket URI";
    let (schema, uri) = uri.split_once("://").ok_or(err_msg)?;
    let secure = if schema.eq_ignore_ascii_case("ws") {
        false
    } else if schema.eq_ignore_ascii_case("wss") {
        true
    } else {
        return Err(err_msg);
    };
    let (addr, path) = uri.split_once('/').unwrap_or((uri, ""));
    Ok((secure, addr, path))
}

impl WebSocket<CLIENT> {
    pub async fn connect(uri: impl AsRef<str>) -> Result<Self> {
        Self::connect_with_headers(uri, [("", ""); 0]).await
    }

    pub async fn connect_with_headers(
        uri: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl FmtHeaderField>,
    ) -> Result<Self> {
        let (secure, addr, path) = parse_ws_uri(uri.as_ref()).map_err(invalid_input)?;
        let port = if addr.contains(':') {
            ""
        } else {
            match secure {
                true => ":443",
                false => ":80",
            }
        };

        let mut stream = BufReader::new(TcpStream::connect(format!("{addr}{port}")).await?);

        let (request, sec_key) = handshake::request(addr, path, headers);
        stream.get_mut().write_all(request.as_bytes()).await?;

        let mut bytes = stream.fill_buf().await?;
        let total_len = bytes.len();

        let header = http::Record::from_raw(&mut bytes).map_err(invalid_data)?;
        if header.schema != b"HTTP/1.1 101 Switching Protocols" {
            return proto_err("Invalid HTTP response");
        }

        if header
            .get_sec_ws_accept()
            .ok_or_else(|| invalid_data("Couldn't get `Accept-Key` from response"))?
            != handshake::accept_key_from(sec_key).as_bytes()
        {
            return proto_err("Invalid accept key");
        }

        let remaining = bytes.len();
        stream.consume(total_len - remaining);

        Ok(Self {
            stream,
            len: 0,
            fin: true,
            on_event: Box::new(|_| Ok(())),
        })
    }

    pub async fn recv(&mut self) -> Result<Data> {
        let ty = cls_if_err!(self, self.read_data_frame_header().await)?;
        Ok(client::Data { ty, ws: self })
    }
}

pub struct Data<'a> {
    pub ty: DataType,
    pub(crate) ws: &'a mut WebSocket<CLIENT>,
}

impl Data<'_> {
    async fn _read_next_frag(&mut self) -> Result<()> {
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
