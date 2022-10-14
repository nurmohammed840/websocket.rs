mod convert;
// pub mod frame;
pub mod utils;

/// ### Data Frame Header
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
#[derive(Debug, PartialEq, Clone)]
pub struct Header {
    /// Indicates that this is the final fragment in a message.  The first
    /// fragment MAY also be the final fragment.
    pub fin: bool,

    /// MUST be `false` unless an extension is negotiated that defines meanings
    /// for non-zero values.  If a nonzero value is received and none of
    /// the negotiated extensions defines the meaning of such a nonzero
    /// value, the receiving endpoint MUST _Fail the WebSocket Connection_.
    pub rsv: Rsv,

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
    pub mask: Option<[u8; 4]>,
}

/// Defines the interpretation of the "Payload data".  If an unknown
/// opcode is received, the receiving endpoint MUST _Fail the WebSocket Connection_.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    /// The FIN and opcode fields work together to send a message split up into separate frames. This is called message fragmentation.
    ///
    /// ```txt
    /// Client: FIN=1, opcode=0x1, msg="hello"
    /// Server: (process complete message immediately) Hi.
    /// Client: FIN=0, opcode=0x1, msg="and a"
    /// Server: (listening, new message containing text started)
    /// Client: FIN=0, opcode=0x0, msg="happy new"
    /// Server: (listening, payload concatenated to previous message)
    /// Client: FIN=1, opcode=0x0, msg="year!"
    /// Server: (process complete message) Happy new year to you too!
    /// ```
    ///
    /// ### Note
    ///
    /// - Control frames MAY be injected in the middle of
    ///   a fragmented message.  Control frames themselves MUST NOT be
    ///   fragmented. An endpoint MUST be capable of handling control frames in the
    ///   middle of a fragmented message.
    ///
    Continue = 0,

    Text = 1,
    Binary = 2,

    // 3-7 are reserved for further non-control frames.
    
    /// - The Close frame MAY contain a body that indicates a reason for closing.
    ///
    /// - If there is a body, the first two bytes of the body MUST be a 2-byte unsigned integer (in network byte order: Big Endian)
    ///   representing a status code with value /code/ defined in [Section 7.4](https://datatracker.ietf.org/doc/html/rfc6455#section-7.4). Following the 2-byte integer,
    ///
    /// - Close frames sent from client to server must be masked.
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
    Close = 8,

    /// A Ping frame MAY include "Application data".
    /// Unless it already received a Close frame.  It SHOULD respond with Pong frame as soon as is practical.
    ///
    /// A Ping frame may serve either as a keepalive or as a means to verify that the remote endpoint is still responsive.
    Ping = 9,

    /// A Pong frame sent in response to a Ping frame must have identical
    /// "Application data" as found in the message body of the Ping frame being replied to.
    ///
    /// If an endpoint receives a Ping frame and has not yet sent Pong frame(s) in response to previous Ping frame(s), the endpoint MAY
    /// elect to send a Pong frame for only the most recently processed Ping frame.
    ///
    ///  A Pong frame MAY be sent unsolicited.  This serves as a unidirectional heartbeat.  A response to an unsolicited Pong frame is not expected.
    Pong = 10,

    // 11-15 are reserved for further control frames
}

impl Opcode {
    /// Whether the opcode indicates a control frame.
    pub fn is_control(self) -> bool {
        self as u8 >= 8
    }
}

/// When closing an established connection an endpoint MAY indicate a reason for closure.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum CloseCode {
    /// The purpose for which the connection was established has been fulfilled
    Normal = 1000,
    /// Server going down or a browser having navigated away from a page
    Away = 1001,
    /// An endpoint is terminating the connection due to a protocol error.
    ProtocolError = 1002,
    /// It has received a type of data it cannot accept
    Unsupported = 1003,

    // reserved 1004
    /// MUST NOT be set as a status code in a Close control frame by an endpoint.
    ///
    /// No status code was actually present.
    NoStatusRcvd = 1005,
    /// MUST NOT be set as a status code in a Close control frame by an endpoint.
    ///
    /// Connection was closed abnormally.
    Abnormal = 1006,
    /// Application has received data within a message that was not consistent with the type of the message.
    InvalidPayload = 1007,
    /// This is a generic status code that can be returned when there is no other more suitable status code.
    PolicyViolation = 1008,
    /// Message that is too big for it to process.
    MessageTooBig = 1009,
    /// It has expected the server to negotiate one or more extension.
    MandatoryExt = 1010,
    /// The server has encountered an unexpected condition that prevented it from fulfilling the request.
    InternalError = 1011,
    /// MUST NOT be set as a status code in a Close control frame by an endpoint.
    ///
    /// The connection was closed due to a failure to perform a TLS handshake.
    TLSHandshake = 1015,
}

/// Rsv are used for extensions.
#[derive(Default, PartialEq, Clone)]
pub struct Rsv(pub u8);

impl Rsv {
    /// The first bit of the RSV field.
    pub fn rsv1(&self) -> bool {
        self.0 & 0b_100_0000 != 0
    }

    /// The second bit of the RSV field.
    pub fn rsv2(&self) -> bool {
        self.0 & 0b__10_0000 != 0
    }

    /// The third bit of the RSV field.
    pub fn rsv3(&self) -> bool {
        self.0 & 0b___1_0000 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::apply_mask;
    use bin_layout::{Decoder, Encoder};

    fn read_slice<'a>(remaining: &mut &'a [u8], len: usize) -> &'a [u8] {
        let slice = &remaining[..len];
        *remaining = &remaining[len..];
        slice
    }

    #[test]
    fn unmasked_text_message() {
        let mut c = [0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f].as_slice();
        assert_eq!(
            Header::decoder(&mut c).unwrap(),
            Header {
                fin: true,
                rsv: Rsv(0),
                opcode: Opcode::Text,
                len: 5,
                mask: None
            }
        );
        assert_eq!(c, b"Hello");
    }

    #[test]
    fn masked_text_message() {
        let mut c = [
            0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
        ]
        .as_slice();

        let header = Header {
            fin: true,
            rsv: Rsv(0),
            opcode: Opcode::Text,
            len: 5,
            mask: Some([55, 250, 33, 61]),
        };
        assert_eq!(Header::decoder(&mut c).unwrap(), header);

        let mut payload = c.to_vec();
        apply_mask(header.mask.unwrap(), &mut payload);
        assert_eq!(payload, b"Hello");
    }

    #[test]
    fn fragmented_unmasked_text_message() {
        let mut c = [
            0x01, 0x03, 0x48, 0x65, 0x6c, // fragmented frame
            0x80, 0x02, 0x6c, 0x6f, // final frame
        ]
        .as_slice();
        assert_eq!(
            Header::decoder(&mut c).unwrap(),
            Header {
                fin: false,
                rsv: Rsv(0),
                opcode: Opcode::Text,
                len: 3,
                mask: None
            }
        );
        assert_eq!(read_slice(&mut c, 3), b"Hel");
        assert_eq!(
            Header::decoder(&mut c).unwrap(),
            Header {
                fin: true,
                rsv: Rsv(0),
                opcode: Opcode::Continue,
                len: 2,
                mask: None
            }
        );
        assert_eq!(c, b"lo");
    }

    #[test]
    fn unmasked_ping_req_and_masked_pong_res() {
        let mut c = [
            0x89, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, // unmasked ping request
            0x8a, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58, // masked pong response
        ]
        .as_slice();
        let unmask_ping_req = Header {
            fin: true,
            rsv: Rsv(0),
            opcode: Opcode::Ping,
            len: 5,
            mask: None,
        };
        assert_eq!(Header::decoder(&mut c).unwrap(), unmask_ping_req);
        assert_eq!(read_slice(&mut c, 5), b"Hello");

        let masked_pong_res = Header {
            fin: true,
            rsv: Rsv(0),
            opcode: Opcode::Pong,
            len: 5,
            mask: Some([55, 250, 33, 61]),
        };
        assert_eq!(Header::decoder(&mut c).unwrap(), masked_pong_res);
        let mut payload = c.to_vec();
        apply_mask(masked_pong_res.mask.unwrap(), &mut payload);
        assert_eq!(payload, b"Hello");
    }

    #[test]
    fn test_payload_len() {
        let mut header = Header {
            fin: true,
            rsv: Rsv(0),
            opcode: Opcode::Binary,
            len: 0,
            mask: None,
        };

        header.len = 256;
        assert_eq!(header.encode(), [130, 126, 1, 0]);

        header.len = 65536;
        assert_eq!(header.encode(), [130, 127, 0, 0, 0, 0, 0, 1, 0, 0]);
    }
}
