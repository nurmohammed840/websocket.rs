use crate::{frame::Frame, *};
use bin_layout::*;
use CloseCode::*;
use Opcode::*;

const MSB: u8 = 0b_1000_0000;

// =================================================================================

impl Encoder for Header {
    fn size_hint(&self) -> usize {
        14 // Max header size
    }
    fn encoder(self, c: &mut impl Array<u8>) {
        c.push(((self.fin as u8) << 7) | self.rsv.0 | self.opcode as u8);

        let b2 = (self.mask.is_some() as u8) << 7;
        let len = self.payload_len;

        if len < 126 {
            c.push(b2 | len as u8);
        } else if len < 65536 {
            c.push(b2 | 126);
            (len as u16).encoder(c);
        } else {
            c.push(b2 | 127);
            (len as u64).encoder(c);
        }

        if let Some(keys) = self.mask {
            c.extend_from_slice(keys);
        }
    }
}

impl<E: Error> Decoder<'_, E> for Header {
    fn decoder(c: &mut Cursor<&[u8]>) -> Result<Self, E> {
        let [b1, b2] = <[u8; 2]>::decoder(c)?;

        let fin = b1 & MSB != 0;
        let opcode = match b1 & 0b_1111 {
            0 => Continue,
            1 => Text,
            2 => Binary,
            8 => Close,
            9 => Ping,
            10 => Pong,
            // If an unknown opcode is received, the receiving endpoint MUST _Fail the WebSocket Connection_.
            _ => return Err(E::invalid_data()),
        };
        let data_len = b2 & 0b_111_1111;
        let payload_len = if opcode.is_control() {
            // Control frames MUST NOT be fragmented.
            // All control frames MUST have a payload length of 125 bytes or less
            if !fin || data_len > 125 {
                return Err(E::invalid_data());
            }
            data_len as usize
        } else {
            match data_len {
                126 => u16::decoder(c)? as usize,
                127 => u64::decoder(c)? as usize,
                len => len as usize,
            }
        };
        Ok(Self {
            fin,
            rsv: Rsv(b1 & 0b_111_0000),
            opcode,
            payload_len,
            mask: if b2 & MSB != 0 {
                Some(<[u8; 4]>::decoder(c)?)
            } else {
                None
            },
        })
    }
}

// =================================================================================

fn encode_payload(payload: &[u8], mask: Option<[u8; 4]>, arr: &mut impl Array<u8>) {
    match mask {
        Some(keys) => {
            let len = arr.len();
            let total_len = len + payload.len();
            arr.ensure_capacity(total_len);
            unsafe {
                let end = arr.as_mut().as_mut_ptr().add(len);
                for (i, byte) in payload.into_iter().enumerate() {
                    end.add(i).write(byte ^ keys.get_unchecked(i % 4));
                }
                arr.set_len(total_len);
            }
        }
        None => arr.extend_from_slice(payload),
    }
}

impl Encoder for Frame<&[u8]> {
    fn encoder(self, arr: &mut impl Array<u8>) {
        let mask = self.header.mask.clone(); // cloning `mask` is cheap
        self.header.encoder(arr);
        encode_payload(self.payload, mask, arr);
    }
}

impl<T> Encoder for Frame<(T, &[u8])>
where
    T: Encoder, // CloseCode
{
    fn encoder(self, arr: &mut impl Array<u8>) {
        let mask = self.header.mask.clone();
        self.header.encoder(arr);
        self.payload.0.encoder(arr);
        encode_payload(self.payload.1, mask, arr);
    }
}

// =================================================================================

impl Encoder for CloseCode {
    fn encoder(self, c: &mut impl Array<u8>) {
        (self as u16).encoder(c)
    }
}

impl<E: Error> Decoder<'_, E> for CloseCode {
    fn decoder(c: &mut Cursor<&[u8]>) -> Result<Self, E> {
        Ok(match u16::decoder(c)? {
            1000 => Normal,
            1001 => Away,
            1002 => ProtocolError,
            1003 => Unsupported,
            1005 => NoStatusRcvd,
            1006 => Abnormal,
            1007 => InvalidPayload,
            1008 => PolicyViolation,
            1009 => MessageTooBig,
            1010 => MandatoryExt,
            1011 => InternalError,
            1015 => TLSHandshake,
            _ => return Err(E::invalid_data()),
        })
    }
}

impl Error for CloseCode {
    fn insufficient_bytes() -> Self {
        MessageTooBig
    }
    fn invalid_data() -> Self {
        PolicyViolation
    }
    fn utf8_err(_: std::str::Utf8Error) -> Self {
        InvalidPayload
    }
}

// ================================================================

impl std::fmt::Debug for Rsv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#b}", (self.0 >> 4) & 0b111)
    }
}