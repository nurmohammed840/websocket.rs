use super::*;
use http::Header;
use std::{fmt::Display, sync::Arc};
use tokio::{
    io::BufReader,
    net::{TcpStream, ToSocketAddrs},
};
use tokio_rustls::{
    client::TlsStream,
    rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore},
    TlsConnector,
};

/// Unencrypted [WebSocket] client.
pub type WS = WebSocket<CLIENT, BufReader<TcpStream>>;

/// Encrypted [WebSocket] client.
pub type WSS = WebSocket<CLIENT, BufReader<TlsStream<TcpStream>>>;

impl WS {
    /// establishe a websocket connection to a remote address.
    pub async fn connect<A>(addr: A, path: impl AsRef<str>) -> Result<Self>
    where
        A: ToSocketAddrs + Display,
    {
        Self::connect_with_headers(addr, path, [("", ""); 0]).await
    }

    /// establishes a connection with headers
    pub async fn connect_with_headers(
        addr: impl ToSocketAddrs + Display,
        path: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl Header>,
    ) -> Result<Self> {
        let host = addr.to_string();
        let mut stream = BufReader::new(TcpStream::connect(addr).await?);
        handshake(&mut stream, &host, path.as_ref(), headers).await?;
        Ok(Self::from(stream))
    }
}

impl WSS {
    /// establishe a secure websocket connection to a remote address.
    pub async fn connect<A>(addr: A, path: impl AsRef<str>) -> Result<Self>
    where
        A: ToSocketAddrs + Display,
    {
        Self::connect_with_headers(addr, path, [("", ""); 0]).await
    }

    /// establishes a secure connection with headers
    pub async fn connect_with_headers(
        addr: impl ToSocketAddrs + Display,
        path: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl Header>,
    ) -> Result<Self> {
        let host = addr.to_string();
        // `TcpStream::connect` also validate `addr`, don't move this line.
        let tcp_stream = TcpStream::connect(addr).await?;

        let mut root_store = RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let domain = match host.rsplit_once(':').unwrap().0.try_into() {
            Ok(server_name) => server_name,
            Err(dns_name_err) => err!(dns_name_err),
        };

        let connector = TlsConnector::from(Arc::new(config));
        let mut stream = BufReader::new(connector.connect(domain, tcp_stream).await?);
        handshake(&mut stream, &host, path.as_ref(), headers).await?;
        Ok(Self::from(stream))
    }
}
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

async fn handshake<IO, I, H>(stream: &mut IO, host: &str, path: &str, headers: I) -> Result<()>
where
    IO: Unpin + AsyncBufRead + AsyncWrite,
    I: IntoIterator<Item = H>,
    H: Header,
{
    let (request, sec_key) = handshake::request(host, path, headers);
    stream.write_all(request.as_bytes()).await?;

    let mut bytes = stream.fill_buf().await?;
    let mut amt = bytes.len();

    pub fn http_err(msg: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, msg)
    }

    let header = http::Http::parse(&mut bytes).map_err(http_err)?;
    if header.schema != b"HTTP/1.1 101 Switching Protocols" {
        err!("invalid http response");
    }

    if header
        .get_sec_ws_accept()
        .ok_or_else(|| http_err("couldn't get `Accept-Key` from http response"))?
        != handshake::accept_key_from(sec_key).as_bytes()
    {
        err!("invalid websocket accept key");
    }
    amt -= bytes.len();
    stream.consume(amt);
    Ok(())
}

// impl<IO: Unpin + AsyncRead + AsyncWrite> WebSocket<CLIENT, IO> {
//     async fn handshake<I, H>(&mut self, host: &str, path: &str, headers: I) -> Result<()>
//     where
//         I: IntoIterator<Item = H>,
//         H: Header,
//     {
//         let (request, sec_key) = handshake::request(host, path, headers);
//         self.stream.write_all(request.as_bytes()).await?;

//         let mut bytes = self.stream.fill_buf().await?;
//         let mut amt = bytes.len();

//         pub fn http_err(msg: &str) -> std::io::Error {
//             std::io::Error::new(std::io::ErrorKind::Other, msg)
//         }

//         let header = http::Http::parse(&mut bytes).map_err(http_err)?;
//         if header.schema != b"HTTP/1.1 101 Switching Protocols" {
//             err!("invalid http response");
//         }

//         if header
//             .get_sec_ws_accept()
//             .ok_or_else(|| http_err("couldn't get `Accept-Key` from http response"))?
//             != handshake::accept_key_from(sec_key).as_bytes()
//         {
//             err!("invalid websocket accept key");
//         }

//         amt -= bytes.len();
//         self.stream.consume(amt);
//         Ok(())
//     }
// }

impl<IO: Unpin + AsyncRead + AsyncWrite> WebSocket<CLIENT, IO> {
    /// reads [Data] from websocket stream.
    #[inline]
    pub async fn recv(&mut self) -> Result<Data<IO>> {
        let ty = cls_if_err!(self, self.read_data_frame_header().await)?;
        Ok(client::Data { ty, ws: self })
    }
}

/// It represent a single websocket message.
pub struct Data<'a, Stream> {
    /// A [DataType] value indicating the type of the data.
    pub ty: DataType,
    pub(crate) ws: &'a mut WebSocket<CLIENT, Stream>,
}

impl<IO: Unpin + AsyncRead + AsyncWrite> Data<'_, IO> {
    #[inline]
    async fn _read_next_frag(&mut self) -> Result<()> {
        self.ws.read_fragmented_header().await
    }

    #[inline]
    async fn _read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = buf.len().min(self.ws.len);
        if len > 0 {
            len = self.ws.stream.read(&mut buf[..len]).await?;
            self.ws.len -= len;
        }
        Ok(len)
    }
}

default_impl_for_data!();
