#![allow(warnings)]
use crate::*;
use bin_layout::*;

pub struct Frame<T> {
    pub header: Header,
    pub payload: T,
}

impl Header {
    fn control(opcode: Opcode, len: usize, mask: Option<[u8; 4]>) -> Self {
        Self {
            fin: true,
            rsv: Rsv(0),
            opcode,
            len,
            mask,
        }
    }
}

fn trim_control_payload(msg: &[u8]) -> &[u8] {
    unsafe { msg.get_unchecked(..msg.len().min(123)) }
}

impl<T> Frame<T> {
    pub fn ping<'a>(msg: &'a [u8]) -> Frame<&'a [u8]> {
        let payload = trim_control_payload(msg);
        Frame {
            header: Header::control(Opcode::Ping, payload.len(), None),
            payload,
        }
    }
}

type CloseFrame<'a> = Frame<(CloseCode, &'a [u8])>;


