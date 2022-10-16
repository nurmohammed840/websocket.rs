// use super::*;

macro_rules! default_read_exact {
    ($this: expr, $buf: expr) => ({
        while !$buf.is_empty() {
            match $this.read($buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = $buf;
                    $buf = &mut tmp[n..];
                }
                Err(e) => return Err(e),
            }
        }
        if !$buf.is_empty() {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof,"failed to fill whole buffer",))
        } else {
            Ok(())
        }
    });
}
pub(crate) use default_read_exact;