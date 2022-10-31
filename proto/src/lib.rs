#![doc = include_str!("../README.md")]
// mod convert;

pub mod handshake;

mod close_code;
mod opcode;
mod rsv;

pub use close_code::*;
pub use opcode::*;
pub use rsv::*;

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

    pub opcode: u8,

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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::utils::apply_mask;
//     use bin_layout::{Decoder, Encoder};

//     fn read_slice<'a>(remaining: &mut &'a [u8], len: usize) -> &'a [u8] {
//         let slice = &remaining[..len];
//         *remaining = &remaining[len..];
//         slice
//     }

//     #[test]
//     fn unmasked_text_message() {
//         let mut c = [0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f].as_slice();
//         assert_eq!(
//             Header::decoder(&mut c).unwrap(),
//             Header {
//                 fin: true,
//                 rsv: Rsv(0),
//                 opcode: Opcode::Text,
//                 len: 5,
//                 mask: None
//             }
//         );
//         assert_eq!(c, b"Hello");
//     }

//     #[test]
//     fn masked_text_message() {
//         let mut c = [
//             0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
//         ]
//         .as_slice();

//         let header = Header {
//             fin: true,
//             rsv: Rsv(0),
//             opcode: Opcode::Text,
//             len: 5,
//             mask: Some([55, 250, 33, 61]),
//         };
//         assert_eq!(Header::decoder(&mut c).unwrap(), header);

//         let mut payload = c.to_vec();
//         apply_mask(header.mask.unwrap(), &mut payload);
//         assert_eq!(payload, b"Hello");
//     }

//     #[test]
//     fn fragmented_unmasked_text_message() {
//         let mut c = [
//             0x01, 0x03, 0x48, 0x65, 0x6c, // fragmented frame
//             0x80, 0x02, 0x6c, 0x6f, // final frame
//         ]
//         .as_slice();
//         assert_eq!(
//             Header::decoder(&mut c).unwrap(),
//             Header {
//                 fin: false,
//                 rsv: Rsv(0),
//                 opcode: Opcode::Text,
//                 len: 3,
//                 mask: None
//             }
//         );
//         assert_eq!(read_slice(&mut c, 3), b"Hel");
//         assert_eq!(
//             Header::decoder(&mut c).unwrap(),
//             Header {
//                 fin: true,
//                 rsv: Rsv(0),
//                 opcode: Opcode::Continue,
//                 len: 2,
//                 mask: None
//             }
//         );
//         assert_eq!(c, b"lo");
//     }

//     #[test]
//     fn unmasked_ping_req_and_masked_pong_res() {
//         let mut c = [
//             0x89, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, // unmasked ping request
//             0x8a, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51,
//             0x58, // masked pong response
//         ]
//         .as_slice();
//         let unmask_ping_req = Header {
//             fin: true,
//             rsv: Rsv(0),
//             opcode: Opcode::Ping,
//             len: 5,
//             mask: None,
//         };
//         assert_eq!(Header::decoder(&mut c).unwrap(), unmask_ping_req);
//         assert_eq!(read_slice(&mut c, 5), b"Hello");

//         let masked_pong_res = Header {
//             fin: true,
//             rsv: Rsv(0),
//             opcode: Opcode::Pong,
//             len: 5,
//             mask: Some([55, 250, 33, 61]),
//         };
//         assert_eq!(Header::decoder(&mut c).unwrap(), masked_pong_res);
//         let mut payload = c.to_vec();
//         apply_mask(masked_pong_res.mask.unwrap(), &mut payload);
//         assert_eq!(payload, b"Hello");
//     }

//     #[test]
//     fn test_payload_len() {
//         let mut header = Header {
//             fin: true,
//             rsv: Rsv(0),
//             opcode: Opcode::Binary,
//             len: 0,
//             mask: None,
//         };

//         header.len = 256;
//         assert_eq!(header.encode(), [130, 126, 1, 0]);

//         header.len = 65536;
//         assert_eq!(header.encode(), [130, 127, 0, 0, 0, 0, 0, 1, 0, 0]);
//     }
// }
