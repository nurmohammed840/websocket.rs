use super::*;
use http::{Header, HeaderField};

fn parse_ws_uri(uri: &str) -> std::result::Result<(bool, &str, &str), &'static str> {
    let err_msg = "Invalid Websocket URI";
    let (schema, addr_uri) = uri.split_once("://").ok_or(err_msg)?;
    let secure = if schema.eq_ignore_ascii_case("ws") {
        false
    } else if schema.eq_ignore_ascii_case("wss") {
        true
    } else {
        return Err(err_msg);
    };
    let (addr, path) = addr_uri.split_once("/").unwrap_or((uri, ""));
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
        let port = addr.contains(":").then_some("").unwrap_or(match secure {
            true => ":443",
            false => ":80",
        });

        let mut stream = BufReader::new(TcpStream::connect(format!("{addr}{port}")).await?);

        let (request, sec_key) = handshake::request(addr, path, headers);
        stream.get_mut().write_all(request.as_bytes()).await?;

        let data = stream.fill_buf().await?;
        let total_len = data.len();

        let mut bytes = data
            .strip_prefix(b"HTTP/1.1 101 Switching Protocols")
            .ok_or(invalid_data("Invalid HTTP response"))?;

        let mut header = Header::default();
        header.parse_from_raw(&mut bytes).map_err(invalid_data)?;

        let accept_key = header
            .get_sec_ws_accept_key()
            .ok_or(invalid_data("Couldn't get `Accept-Key` from response"))?;

        if handshake::accept_key_from(sec_key).as_bytes() != accept_key {
            return Err(invalid_data("Invalid accept key"));
        }

        let remaining = bytes.len();
        stream.consume(total_len - remaining);

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


// #[tokio::test]
// async fn test_name() -> Result<()> {
//     let mut ws = Websocket::connect("ws://ws.ifelse.io/").await?;

//     ws.event = Box::new(|ev|{
//         println!("{:?}", ev);
//         Ok(())
//     });

//     ws.send(crate::frame::Ping(b"Hello, World")).await?;

//     // let _ = ws.recv().await?; // ignore first message : Request served by 33ed2ee9

//     let mut data = ws.recv().await?;
//     println!("{:?}", data.ty);

//     let mut buf = vec![];
//     data.read_to_end(&mut buf).await?;
//     println!("{:?}", String::from_utf8(buf));
//     Ok(())
// }
