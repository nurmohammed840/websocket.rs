use super::*;

impl<IO> WebSocket<CLIENT, IO> {
    /// Create a new websocket client instance.
    #[inline]
    pub fn client(stream: IO) -> Self {
        Self::from(stream)
    }
}

#[inline]
async fn footer<IO>(this: &mut WebSocket<CLIENT, IO>, ty: DataType, len: usize) -> Result<Event>
where
    IO: Unpin + AsyncRead,
{
    let mut data = vec![0; len].into_boxed_slice();
    this.stream.read_exact(&mut data).await?;
    Ok(Event::Data {
        ty,
        done: this.done,
        data,
    })
}

impl<IO: Unpin + AsyncRead> WebSocket<CLIENT, IO> {
    /// reads [Event] from websocket stream.
    #[inline]
    pub async fn recv(&mut self) -> Result<Event> {
        if self.is_closed {
            io_err!(NotConnected, "read after close");
        }
        let event = self.header(footer).await;
        if let Ok(Event::Close { .. } | Event::Error(..)) | Err(..) = event {
            self.is_closed = true;
        }
        event
    }
}

// ---------------------------------------------------------------------------------------

use http::Header;
// use tokio::{
//     io::{AsyncBufRead, AsyncBufReadExt, BufReader},
//     net::{TcpStream, ToSocketAddrs},
// };

impl<IO: Unpin + AsyncRead + AsyncWrite> WebSocket<CLIENT, IO> {
    ///
    pub async fn handshake(&mut self, host: &str, path: &str) -> Result<()> {
        self.handshake_with_headers(host, path, [("", ""); 0]).await
    }

    ///
    pub async fn handshake_with_headers<I, H>(
        &mut self,
        host: &str,
        path: &str,
        headers: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = H>,
        H: Header,
    {
        let (request, _sec_key) = handshake::request(host, path, headers);
        self.stream.write_all(request.as_bytes()).await?;

        Ok(())
    }
}

// struct RawHttp {
//     buf: Vec<u8>,
//     end_pattern: [u8; 4],
// }

// fn extract_http_headers(response: &str) -> Vec<u8> {
//     let mut headers = Vec::new();
//     let mut header_found = false;

//     for byte in response.bytes() {
//         if byte == b'\r' {
//             header_found = true;
//         } else if byte == b'\n' && header_found {
//             headers.push(b'\r');
//             headers.push(b'\n');
//             header_found = false;
//         } else {
//             header_found = false;
//         }
//         headers.push(byte);
//     }

//     headers
// }

// async fn raw_http(reader: &mut impl AsyncRead) {
//     let raw_http = vec![0];
//     let end_pattern = [0; 4];
// }

// async fn handshake<IO, I, H>(stream: &mut IO, host: &str, path: &str, headers: I) -> Result<()>
// where
//     IO: Unpin + AsyncBufRead + tokio::io::AsyncWrite,
//     I: IntoIterator<Item = H>,
//     H: Header,
// {
//     let (request, sec_key) = handshake::request(host, path, headers);
//     stream.write_all(request.as_bytes()).await?;

//     let mut bytes = stream.fill_buf().await?;
//     let mut amt = bytes.len();

//     pub fn http_err(msg: &str) -> std::io::Error {
//         std::io::Error::new(std::io::ErrorKind::Other, msg)
//     }

//     let header = http::Http::parse(&mut bytes).map_err(http_err)?;
//     if header.schema != b"HTTP/1.1 101 Switching Protocols" {
//         err!("invalid http response");
//     }

//     if header
//         .get("sec-websocket-accept")
//         .ok_or_else(|| http_err("couldn't get `Accept-Key` from http response"))?
//         != handshake::accept_key_from(sec_key).as_bytes()
//     {
//         err!("invalid websocket accept key");
//     }
//     amt -= bytes.len();
//     stream.consume(amt);
//     Ok(())
// }

// ---------------------------------------------------------------------------------------

#[cfg(feature = "tls")]
use tokio::net::TcpStream;

#[cfg(feature = "tls")]
use tokio_rustls::{
    client::TlsStream,
    rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore},
    TlsConnector,
};

#[cfg(feature = "tls")]
/// Encrypted [WebSocket] client.
pub type WSS = WebSocket<CLIENT, TlsStream<TcpStream>>;

#[cfg(feature = "tls")]
///
pub async fn secure_connect<Addr>(addr: Addr) -> Result<WSS>
where
    Addr: tokio::net::ToSocketAddrs + std::fmt::Display,
{
    let host = addr.to_string();
    let stream = TcpStream::connect(addr).await?;

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

    let connector = TlsConnector::from(std::sync::Arc::new(config));

    let domain = match host.rsplit_once(':').unwrap().0.try_into() {
        Ok(server_name) => server_name,
        Err(err) => io_err!(InvalidData, err),
    };
    let stream = connector.connect(domain, stream).await?;
    Ok(WebSocket::client(stream))
}
