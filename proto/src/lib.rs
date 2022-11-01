#![doc = include_str!("../README.md")]

// mod header;
// pub use rsv::*;
// mod rsv;

mod close_code;
mod opcode;

pub mod frame;
pub mod handshake;

pub use close_code::*;
pub use opcode::*;

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
