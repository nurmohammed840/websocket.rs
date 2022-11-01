use crate::Opcode;

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
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Header {
    /// Indicates that this is the final fragment in a message.  The first
    /// fragment MAY also be the final fragment.
    pub fin: bool,

    /// MUST be `false` unless an extension is negotiated that defines meanings
    /// for non-zero values.  If a nonzero value is received and none of
    /// the negotiated extensions defines the meaning of such a nonzero
    /// value, the receiving endpoint MUST _Fail the WebSocket Connection_.
    pub rsv: u8,

    pub opcode: Opcode,

    /// Length of the "Payload data" in bytes.
    pub len: usize,

    /// Defines whether the "Payload data" is masked.  If set to 1, a
    /// masking key is present in masking-key, and this is used to unmask
    /// the "Payload data" as per [Section 5.3](https://datatracker.ietf.org/doc/html/rfc6455#section-5.3).  All frames sent from
    /// client to server have this bit set to 1.
    ///
    /// ### Required for client
    ///
    /// A client MUST mask all frames that it sends to the server. (Note
    /// that masking is done whether or not the WebSocket Protocol is running
    /// over TLS.)  The server MUST close the connection upon receiving a
    /// frame that is not masked.
    ///
    /// A server MUST NOT mask any frames that it sends to the client.
    pub is_masked: bool,
}

fn read_buf<const N: usize>(r: &mut impl std::io::Read) -> Result<[u8; N], &'static str> {
    let mut buf = [0; N];
    r.read_exact(&mut buf).map_err(|_| "Unexpected EOF")?;
    Ok(buf)
}

impl Header {
    pub fn parse(reader: &mut impl std::io::Read) -> Result<Self, &'static str> {
        let [b1, b2] = read_buf(reader)?;

        let fin = b1 & 0b1000_0000 != 0;
        let rsv = b1 & 0b111_0000;
        let opcode = Opcode::try_from(b1 & 0b1111)?;
        let len = (b2 & 0b111_1111) as usize;
        let is_masked = b2 & 0b1000_0000 != 0;

        let len = if opcode.is_control() {
            if !fin {
                return Err("Control frame MUST NOT be fragmented");
            }
            if len > 125 {
                return Err("Control frame MUST have a payload length of 125 bytes or less");
            }
            len as usize
        } else {
            if !fin && len == 0 {
                return Err("Fragment length shouldn't be zero");
            }
            match len {
                126 => u16::from_be_bytes(read_buf(reader)?) as usize,
                127 => u64::from_be_bytes(read_buf(reader)?) as usize,
                len => len,
            }
        };
        Ok(Self {
            fin,
            rsv,
            opcode,
            len,
            is_masked,
        })
    }
}
