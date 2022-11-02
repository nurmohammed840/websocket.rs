use super::*;
use http::FmtHeader;
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

pub type WS = WebSocket<CLIENT, BufReader<TcpStream>>;
pub type WSS = WebSocket<CLIENT, BufReader<TlsStream<TcpStream>>>;

impl WS {
    #[inline]
    pub async fn connect<A>(addr: A, path: impl AsRef<str>) -> Result<Self>
    where
        A: ToSocketAddrs + Display,
    {
        Self::connect_with_headers(addr, path, [("", ""); 0]).await
    }

    pub async fn connect_with_headers(
        addr: impl ToSocketAddrs + Display,
        path: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl FmtHeader>,
    ) -> Result<Self> {
        let host = addr.to_string();
        let mut ws = Self::from(BufReader::new(TcpStream::connect(addr).await?));
        ws.handshake(&host, path.as_ref(), headers).await?;
        Ok(ws)
    }
}

impl WSS {
    #[inline]
    pub async fn connect<A>(addr: A, path: impl AsRef<str>) -> Result<Self>
    where
        A: ToSocketAddrs + Display,
    {
        Self::connect_with_headers(addr, path, [("", ""); 0]).await
    }

    pub async fn connect_with_headers(
        addr: impl ToSocketAddrs + Display,
        path: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl FmtHeader>,
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
            Err(msg) => return proto_err(msg),
        };
        
        let connector = TlsConnector::from(Arc::new(config));
        let stream = BufReader::new(connector.connect(domain, tcp_stream).await?);
        let mut wss = Self::from(stream);

        wss.handshake(&host, path.as_ref(), headers).await?;
        Ok(wss)
    }
}

impl<IO: Unpin + AsyncBufRead + AsyncWrite> WebSocket<CLIENT, IO> {
    async fn handshake(
        &mut self,
        host: &str,
        path: &str,
        headers: impl IntoIterator<Item = impl FmtHeader>,
    ) -> Result<()> {
        let (request, sec_key) = handshake::request(host, path, headers);
        self.stream.write_all(request.as_bytes()).await?;

        let mut bytes = self.stream.fill_buf().await?;
        let mut amt = bytes.len();

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

        amt -= bytes.len();
        self.stream.consume(amt);
        Ok(())
    }
}

impl<RW: Unpin + AsyncBufRead + AsyncWrite> WebSocket<CLIENT, RW> {
    #[inline]
    pub async fn recv(&mut self) -> Result<Data<RW>> {
        let ty = cls_if_err!(self, self.read_data_frame_header().await)?;
        Ok(client::Data { ty, ws: self })
    }
}

pub struct Data<'a, Stream> {
    pub ty: DataType,
    pub(crate) ws: &'a mut WebSocket<CLIENT, Stream>,
}

impl<RW: Unpin + AsyncBufRead + AsyncWrite> Data<'_, RW> {
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
