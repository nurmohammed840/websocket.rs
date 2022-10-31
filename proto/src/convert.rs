use crate::*;
use bin_layout::*;
use std::io;

type DynErr = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, DynErr>;

const MSB: u8 = 0b_1000_0000;

impl Encoder for Header {
    fn encoder(&self, c: &mut impl io::Write) -> io::Result<()> {
        (((self.fin as u8) << 7) | self.rsv.0 | self.opcode as u8).encoder(c)?;

        let b2 = (self.mask.is_some() as u8) << 7;
        let len = self.len;

        if len < 126 {
            (b2 | len as u8).encoder(c)?;
        } else if len < 65536 {
            (b2 | 126).encoder(c)?;
            (len as u16).encoder(c)?;
        } else {
            (b2 | 127).encoder(c)?;
            (len as u64).encoder(c)?;
        }
        if let Some(keys) = self.mask {
            c.write_all(&keys)?;
        }
        Ok(())
    }
}

impl Decoder<'_> for Header {
    fn decoder(c: &mut &[u8]) -> Result<Self> {
        use Opcode::*;
        let [b1, b2] = <[u8; 2]>::decoder(c)?;

        let fin = b1 & MSB != 0;
        let opcode = match b1 & 0b_1111 {
            0 => Continue,
            1 => Text,
            2 => Binary,
            8 => Close,
            9 => Ping,
            10 => Pong,
            _ => return Err("Unknown opcode".into()),
        };
        let len = b2 & 0b_111_1111;
        let len = if opcode.is_control() {
            if !fin || len > 125 {
                return Err("Control frames MUST have a payload length of 125 bytes or less and MUST NOT be fragmented".into());
            }
            len as usize
        } else {
            match len {
                126 => u16::decoder(c)? as usize,
                127 => u64::decoder(c)? as usize,
                len => len as usize,
            }
        };
        Ok(Self {
            fin,
            rsv: Rsv(b1 & 0b_111_0000),
            opcode,
            len,
            mask: if b2 & MSB != 0 {
                Some(<[u8; 4]>::decoder(c)?)
            } else {
                None
            },
        })
    }
}

// =================================================================================

impl Encoder for CloseCode {
    fn encoder(&self, c: &mut impl io::Write) -> io::Result<()> {
        (*self as u16).encoder(c)
    }
}

impl Decoder<'_> for CloseCode {
    fn decoder(c: &mut &[u8]) -> Result<Self> {
        use CloseCode::*;
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
            code => return Err(format!("Unknown close code: {code}").into()),
        })
    }
}

// ================================================================

impl std::fmt::Debug for Rsv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#b}", (self.0 >> 4) & 0b111)
    }
}
