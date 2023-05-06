#![allow(clippy::unusual_byte_groupings)]
use std::io::IoSlice;

use crate::*;
use tokio::io::*;

/// WebSocket implementation for both client and server
#[derive(Debug)]
pub struct WebSocket<const SIDE: bool, Stream> {
    /// it is a low-level abstraction that represents the underlying byte stream over which WebSocket messages are exchanged.
    pub stream: Stream,

    /// Maximum allowed payload length in bytes.
    ///
    /// Default: 16 MB
    pub max_payload_len: usize,
    is_closed: bool,
}

impl<const SIDE: bool, W> WebSocket<SIDE, W>
where
    W: Unpin + AsyncWrite,
{
    ///
    pub async fn send_raw(&mut self, fin: bool, opcode: u8, data: &[u8]) -> Result<()> {
        let data_len = data.len();
        let mask_bit = if SERVER == SIDE { 0 } else { 0x80 };

        if SERVER == SIDE && self.stream.is_write_vectored() {
            let mut head = [0; 10];
            let head_len =
                unsafe { write_head(head.as_mut_ptr(), fin, opcode, data_len, mask_bit) };

            let total_len = head_len + data.len();
            let mut slices = [IoSlice::new(&head[..head_len]), IoSlice::new(data)];

            let mut amt = self.stream.write_vectored(&slices).await?;
            if amt == total_len {
                return Ok(());
            }
            // Slighly more optimized than (unstable) write_all_vectored for 2 iovecs.
            while amt <= head_len {
                slices[0] = IoSlice::new(&head[amt..head_len]);
                amt += self.stream.write_vectored(&slices).await?;
            }
            // Header out of the way.
            if amt < total_len && amt > head_len {
                self.stream.write_all(&data[amt - head_len..]).await?;
            }
            Ok(())
        } else {
            let mut buf = Vec::<u8>::with_capacity(if SERVER == SIDE { 10 } else { 14 } + data_len);
            unsafe {
                let dist = buf.as_mut_ptr();
                let len = write_head(dist, fin, opcode, data_len, mask_bit);

                let header_len = if SERVER == SIDE {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), dist.add(len), data_len);
                    len
                } else {
                    let mask = rand::random::<u32>().to_ne_bytes();
                    let [a, b, c, d] = mask;
                    dist.add(len).write(a);
                    dist.add(len + 1).write(b);
                    dist.add(len + 2).write(c);
                    dist.add(len + 3).write(d);

                    let dist = dist.add(len + 4);
                    // TODO: Use SIMD wherever possible for best performance
                    for i in 0..data_len {
                        dist.add(i)
                            .write(data.get_unchecked(i) ^ mask.get_unchecked(i & 3));
                    }
                    len + 4
                };
                buf.set_len(header_len + data_len);
            }
            self.stream.write_all(&buf).await
        }
    }

    /// Send message to a endpoint.
    pub async fn send(&mut self, data: impl Message) -> Result<()> {
        let (fin, opcode, data) = data.frame_type();
        self.send_raw(fin, opcode, data).await
    }

    /// Flushes this output stream, ensuring that all intermediately buffered contents reach their destination.
    #[inline]
    pub async fn flash(&mut self) -> Result<()> {
        self.stream.flush().await
    }

    /// - The Close frame MAY contain a body that indicates a reason for closing.
    pub async fn close<T>(mut self, reason: T) -> Result<()>
    where
        T: CloseFrame,
        T::Frame: AsRef<[u8]>,
    {
        self.stream
            .write_all(reason.encode::<SIDE>().as_ref())
            .await?;

        self.stream.flush().await
    }
}

unsafe fn write_head(dist: *mut u8, fin: bool, opcode: u8, data_len: usize, mask_bit: u8) -> usize {
    dist.write(((fin as u8) << 7) | opcode);
    if data_len < 126 {
        dist.add(1).write(mask_bit | data_len as u8);
        2
    } else if data_len < 65536 {
        let [b2, b3] = (data_len as u16).to_be_bytes();
        dist.add(1).write(mask_bit | 126);
        dist.add(2).write(b2);
        dist.add(3).write(b3);
        4
    } else {
        let [b2, b3, b4, b5, b6, b7, b8, b9] = (data_len as u64).to_be_bytes();
        dist.add(1).write(mask_bit | 127);
        dist.add(2).write(b2);
        dist.add(3).write(b3);
        dist.add(4).write(b4);
        dist.add(5).write(b5);
        dist.add(6).write(b6);
        dist.add(7).write(b7);
        dist.add(8).write(b8);
        dist.add(9).write(b9);
        10
    }
}

// ------------------------------------------------------------------------

macro_rules! err { [$msg: expr] => { return Ok(Event::Error($msg)) }; }

#[inline]
pub async fn read_buf<const N: usize, R>(stream: &mut R) -> Result<[u8; N]>
where
    R: Unpin + AsyncRead,
{
    let mut buf = [0; N];
    stream.read(&mut buf).await?;
    Ok(buf)
}

impl<const SIDE: bool, R> WebSocket<SIDE, R>
where
    R: Unpin + AsyncRead,
{
    /// reads [Event] from websocket stream.
    #[inline]
    pub async fn recv(&mut self) -> Result<Event> {
        if self.is_closed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "read after close",
            ));
        }
        let event = self.read_frame().await;
        if let Ok(Event::Close { .. } | Event::Error(..)) | Err(..) = event {
            self.is_closed = true;
        }
        event
    }

    /// ### WebSocket Frame Header
    ///
    /// ```txt
    ///  0                   1                   2                   3
    ///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    /// +-+-+-+-+-------+-+-------------+-------------------------------+
    /// |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
    /// |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
    /// |N|V|V|V|       |S|             |   (if payload len==126/127)   |
    /// | |1|2|3|       |K|             |                               |
    /// +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
    /// |     Extended payload length continued, if payload len == 127  |
    /// + - - - - - - - - - - - - - - - +-------------------------------+
    /// |                               |Masking-key, if MASK set to 1  |
    /// +-------------------------------+-------------------------------+
    /// | Masking-key (continued)       |          Payload Data         |
    /// +-------------------------------- - - - - - - - - - - - - - - - +
    /// :                     Payload Data continued ...                :
    /// + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
    /// |                     Payload Data continued ...                |
    /// +---------------------------------------------------------------+
    /// ```
    #[inline]
    pub async fn read_frame(&mut self) -> Result<Event> {
        let [b1, b2] = read_buf(&mut self.stream).await?;

        let fin = b1 & 0b_1000_0000 != 0;
        let rsv = b1 & 0b_111_0000;
        let opcode = b1 & 0b_1111;
        let len = (b2 & 0b_111_1111) as usize;

        // Defines whether the "Payload data" is masked.  If set to 1, a
        // masking key is present in masking-key, and this is used to unmask
        // the "Payload data" as per [Section 5.3](https://datatracker.ietf.org/doc/html/rfc6455#section-5.3).  All frames sent from
        // client to server have this bit set to 1.
        // let is_masked = b2 & 0b_1000_0000 != 0;

        if rsv != 0 {
            // MUST be `0` unless an extension is negotiated that defines meanings
            // for non-zero values.  If a nonzero value is received and none of
            // the negotiated extensions defines the meaning of such a nonzero
            // value, the receiving endpoint MUST _Fail the WebSocket Connection_.
            err!("reserve bit must be `0`");
        }

        // // A client MUST mask all frames that it sends to the server. (Note
        // // that masking is done whether or not the WebSocket Protocol is running
        // // over TLS.)  The server MUST close the connection upon receiving a
        // // frame that is not masked.
        // //
        // // A server MUST NOT mask any frames that it sends to the client.
        // if SERVER == SIDE {
        //     if !is_masked {
        //         err!("expected masked frame");
        //     }
        // } else if is_masked {
        //     err!("expected unmasked frame");
        // }

        // 3-7 are reserved for further non-control frames.
        if opcode >= 8 {
            if !fin {
                err!("control frame must not be fragmented");
            }
            if len > 125 {
                err!("control frame must have a payload length of 125 bytes or less");
            }
            let msg = self.read_payload(len).await?;
            match opcode {
                8 => Ok(on_close(msg)),
                // 9 => Ok(Event::Ping(msg)),
                // 10 => Ok(Event::Pong(msg)),
                // 11-15 are reserved for further control frames
                _ => err!("unknown opcode"),
            }
        } else {
            let ty = match (opcode, fin) {
                // (2, true) => DataType::Complete(MessageType::Binary),
                (1, true) => DataType::Complete(MessageType::Text),

                // (2, false) => DataType::Fragment(Fragment::Start(MessageType::Binary)),
                // (1, false) => DataType::Fragment(Fragment::Start(MessageType::Text)),
                // (0, false) => DataType::Fragment(Fragment::Next),
                // (0, true) => DataType::Fragment(Fragment::End),
                _ => err!("unknown opcode"),
            };
            let len = match len {
                126 => u16::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                127 => u64::from_be_bytes(read_buf(&mut self.stream).await?) as usize,
                len => len,
            };
            if len > self.max_payload_len {
                err!("payload length exceeded");
            }
            let data = self.read_payload(len).await?;
            Ok(Event::Data { ty, data })
        }
    }

    async fn read_payload(&mut self, len: usize) -> Result<Box<[u8]>> {
        let mut data = vec![0; len].into_boxed_slice();
        if SIDE == SERVER {
            let mask: [u8; 4] = read_buf(&mut self.stream).await?;
            self.stream.read_exact(&mut data).await?;
            // TODO: Use SIMD wherever possible for best performance
            for i in 0..data.len() {
                data[i] ^= mask[i & 3];
            }
        } else {
            self.stream.read_exact(&mut data).await?;
        }
        Ok(data)
    }
}

/// - If there is a body, the first two bytes of the body MUST be a 2-byte unsigned integer (in network byte order: Big Endian)
///   representing a status code with value /code/ defined in [Section 7.4](https:///datatracker.ietf.org/doc/html/rfc6455#section-7.4).
///   Following the 2-byte integer,
///
/// - The application MUST NOT send any more data frames after sending a `Close` frame.
///
/// - If an endpoint receives a Close frame and did not previously send a
///   Close frame, the endpoint MUST send a Close frame in response.  (When
///   sending a Close frame in response, the endpoint typically echos the
///   status code it received.)  It SHOULD do so as soon as practical.  An
///   endpoint MAY delay sending a Close frame until its current message is
///   sent
///
/// - After both sending and receiving a Close message, an endpoint
///   considers the WebSocket connection closed and MUST close the
///   underlying TCP connection.
fn on_close(msg: Box<[u8]>) -> Event {
    let code = msg
        .get(..2)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        .unwrap_or(1000);

    match code {
        1000..=1003 | 1007..=1011 | 1015 | 3000..=3999 | 4000..=4999 => {
            match msg.get(2..).map(|data| String::from_utf8(data.to_vec())) {
                Some(Ok(msg)) => Event::Close {
                    code,
                    reason: msg.into_boxed_str(),
                },
                None => Event::Close {
                    code,
                    reason: "".into(),
                },
                Some(Err(_)) => Event::Error("invalid utf-8 payload"),
            }
        }
        _ => Event::Error("invalid close code"),
    }
}

impl<IO> WebSocket<CLIENT, IO> {
    /// Create a new websocket client instance.
    #[inline]
    pub fn client(stream: IO) -> Self {
        Self::from(stream)
    }
}

impl<IO> WebSocket<SERVER, IO> {
    /// Create a websocket server instance.
    #[inline]
    pub fn server(stream: IO) -> Self {
        Self::from(stream)
    }
}

impl<const SIDE: bool, IO> From<IO> for WebSocket<SIDE, IO> {
    #[inline]
    fn from(stream: IO) -> Self {
        Self {
            stream,
            max_payload_len: 16 * 1024 * 1024,

            is_closed: false,
        }
    }
}
