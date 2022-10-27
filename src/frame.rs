use crate::*;

pub trait Frame {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>);
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

impl TryFrom<u16> for CloseCode {
    type Error = &'static str;
    fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1000 => CloseCode::Normal,
            1001 => CloseCode::Away,
            1002 => CloseCode::ProtocolError,
            1003 => CloseCode::Unsupported,
            1005 => CloseCode::NoStatusRcvd,
            1006 => CloseCode::Abnormal,
            1007 => CloseCode::InvalidPayload,
            1008 => CloseCode::PolicyViolation,
            1009 => CloseCode::MessageTooBig,
            1010 => CloseCode::MandatoryExt,
            1011 => CloseCode::InternalError,
            1015 => CloseCode::TLSHandshake,
            _ => return Err("Unknown close code")
        })
    }
}

pub(crate) struct Close<'a> {
    pub code: u16,
    pub reason: &'a [u8],
}

impl<T: Frame + ?Sized> Frame for &T {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        T::encode::<SIDE>(self, writer)
    }
}

impl<T: Frame + ?Sized> Frame for Box<T> {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        T::encode::<SIDE>(self, writer)
    }
}

impl Frame for str {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        encode::<SIDE, RandMask>(writer, true, 1, self.as_bytes());
    }
}

impl Frame for [u8] {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        encode::<SIDE, RandMask>(writer, true, 2, self);
    }
}

impl Frame for Close<'_> {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        writer.extend_from_slice(&self.code.to_be_bytes());
        encode::<SIDE, RandMask>(writer, true, 8, self.reason);
    }
}

impl Frame for Event<'_> {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        match self {
            Event::Ping(data) => encode::<SIDE, RandMask>(writer, true, 9, data),
            Event::Pong(data) => encode::<SIDE, RandMask>(writer, true, 10, data),
        }
    }
}



#[inline]
fn encode<const SIDE: bool, Mask: RandKeys>(
    writer: &mut Vec<u8>,
    fin: bool,
    opcode: u8,
    data: &[u8],
) {
    let data_len = data.len();
    writer.reserve(if SERVER == SIDE { 10 } else { 14 } + data_len);
    unsafe {
        let filled = writer.len();
        let start = writer.as_mut_ptr().add(filled);

        let mask_bit = if SERVER == SIDE { 0 } else { 0x80 };

        start.write(((fin as u8) << 7) | opcode);
        let len = if data_len < 126 {
            start.add(1).write(mask_bit | data_len as u8);
            2
        } else if data_len < 65536 {
            let [b2, b3] = (data_len as u16).to_be_bytes();
            start.add(1).write(mask_bit | 126);
            start.add(2).write(b2);
            start.add(3).write(b3);
            4
        } else {
            let [b2, b3, b4, b5, b6, b7, b8, b9] = (data_len as u64).to_be_bytes();
            start.add(1).write(mask_bit | 127);
            start.add(2).write(b2);
            start.add(3).write(b3);
            start.add(4).write(b4);
            start.add(5).write(b5);
            start.add(6).write(b6);
            start.add(7).write(b7);
            start.add(8).write(b8);
            start.add(9).write(b9);
            10
        };

        let header_len = if SERVER == SIDE {
            std::ptr::copy_nonoverlapping(data.as_ptr(), start.add(len), data_len);
            len
        } else {
            let mask = Mask::keys();
            let [a, b, c, d] = mask;
            start.add(len).write(a);
            start.add(len + 1).write(b);
            start.add(len + 2).write(c);
            start.add(len + 3).write(d);

            let dist = start.add(len + 4);
            for (index, byte) in data.iter().enumerate() {
                dist.add(index).write(byte ^ mask[index % 4]);
            }
            len + 4
        };
        writer.set_len(filled + header_len + data_len);
    }
}

#[cfg(test)]
mod encode {
    use super::*;
    const DATA: &[u8] = b"Hello";

    struct DefaultMask;
    impl super::RandKeys for DefaultMask {
        fn keys() -> [u8; 4] {
            [55, 250, 33, 61]
        }
    }

    fn encode<const S: bool>(writer: &mut Vec<u8>, fin: bool, opcode: u8, data: &[u8]) {
        super::encode::<S, DefaultMask>(writer, fin, opcode, data);
    }

    #[test]
    fn unmasked_txt_msg() {
        let mut bytes = vec![];
        encode::<SERVER>(&mut bytes, true, 1, DATA);
        assert_eq!(bytes, [0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn masked_txt_msg() {
        let mut bytes = vec![];
        encode::<CLIENT>(&mut bytes, true, 1, DATA);
        assert_eq!(
            bytes,
            [0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58]
        );
    }

    #[test]
    fn fragmented_unmasked_txt_msg() {
        let mut bytes = vec![];
        encode::<SERVER>(&mut bytes, false, 1, b"Hel");
        encode::<SERVER>(&mut bytes, true, 0, b"lo");
        assert_eq!(
            bytes,
            [
                0x01, 0x03, 0x48, 0x65, 0x6c, // fragmented frame
                0x80, 0x02, 0x6c, 0x6f, // final frame
            ]
        );
    }

    #[test]
    fn unmasked_ping_req_and_masked_pong_res() {
        let mut bytes = vec![];
        encode::<SERVER>(&mut bytes, true, 9, DATA);
        encode::<CLIENT>(&mut bytes, true, 10, DATA);
        assert_eq!(
            bytes,
            [
                // unmasked ping request
                0x89, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, //
                // masked pong response
                0x8a, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
            ]
        );
    }
}
