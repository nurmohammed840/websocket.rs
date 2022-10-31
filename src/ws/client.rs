use super::*;
use http::FmtHeader;

impl WebSocket<CLIENT, BufReader<TcpStream>> {
    #[inline]
    pub async fn connect(
        addr: impl ToSocketAddrs + std::fmt::Display,
        path: impl AsRef<str>,
    ) -> Result<Self> {
        Self::connect_with_headers(addr, path, [("", ""); 0]).await
    }

    pub async fn connect_with_headers(
        addr: impl ToSocketAddrs + std::fmt::Display,
        path: impl AsRef<str>,
        headers: impl IntoIterator<Item = impl FmtHeader>,
    ) -> Result<Self> {
        let host = addr.to_string();
        let mut stream = BufReader::new(TcpStream::connect(addr).await?);

        let (request, sec_key) = handshake::request(host, path, headers);
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

        Ok(Self::from(stream))
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

// ----------------------------------------------------------------------

// use std::sync::Arc;
// use tokio_rustls::{
//     rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore},
//     TlsConnector,
// };

// async fn _k() -> Result<()> {
//     let mut root_store = RootCertStore::empty();
//     root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
//         OwnedTrustAnchor::from_subject_spki_name_constraints(
//             ta.subject,
//             ta.spki,
//             ta.name_constraints,
//         )
//     }));

//     let config = ClientConfig::builder()
//         .with_safe_defaults()
//         .with_root_certificates(root_store)
//         .with_no_client_auth();

//     let connector = TlsConnector::from(Arc::new(config));

//     let mut stream = connector.connect(
//         "example.com".try_into().expect("invalid DNS name"),
//         TcpStream::connect("example.com:443").await?,
//     );
//     let a = stream.await?;
//     Ok(())
// }
