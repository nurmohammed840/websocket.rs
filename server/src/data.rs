#![allow(warnings)]
use super::*;

#[derive(Debug, Clone, Copy)]
pub enum DataType {
    Text,
    Binary,
}

pub struct Data<'a> {
    pub(super) fin: bool,
    pub(super) len: usize,
    pub(super) ty: DataType,
    pub(super) mask: Cycle<IntoIter<u8, 4>>,
    pub(super) stream: &'a mut BufferReader<TcpStream>,
}

impl<'a> Read for Data<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.len > 0 {
            let data = self.stream.fill_buf()?;
            if data.is_empty() {
                return err::conn_aborted();
            }

            let amt = data.len().min(self.len);
            unsafe { data.get_unchecked(..amt) }
                .iter()
                .zip(&mut self.mask)
                .zip(buf.iter_mut())
                .for_each(|((byte, key), dist)| *dist = byte ^ key);

            self.len -= amt;
            self.stream.consume(amt);
        } else if !self.fin {
            let (fin, opcode, len, mask) = recv_header(&mut self.stream)?;
            match opcode {
                Opcode::Continue => {
                    self.fin = fin;
                    self.len = len;
                    self.mask = mask;
                    todo!()
                },
                Opcode::Text | Opcode::Binary => {
                    return err::proto(&format!("Expected fragment, But got {opcode:?}"))
                }
                Opcode::Close => todo!(),
                Opcode::Ping => todo!(),
                Opcode::Pong => todo!(),
            }
        } else {
            return Ok(0);
        }
        Ok(0)
    }
}

impl<'a> Data<'a> {
    pub fn ty(&self) -> DataType {
        self.ty
    }

    pub fn send(&mut self, _msg: impl Into<String>) {}
}

impl From<Opcode> for DataType {
    fn from(opcode: Opcode) -> Self {
        match opcode {
            Opcode::Text => DataType::Text,
            Opcode::Binary => DataType::Binary,
            _ => unreachable!(),
        }
    }
}

/*
let mut remain = header.payload_len;
while remain > 0 {
    let data = match self.stream.fill_buf() {
        Ok(data) => data,
        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
        Err(e) => return Err(e),
    };
    if data.is_empaty() {
        break;
    }
    let amt = data.len().min(remain);
    let _ = writer.write_all(&data[..amt]); // ignore any error
    remain -= amt;
    self.stream.consume(amt);
}
*/
