use super::*;

#[inline]
pub async fn read_buf<const N: usize, R>(stream: &mut R) -> Result<[u8; N]>
where
    R: Unpin + AsyncRead,
{
    let mut buf = [0; N];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

#[inline]
pub fn apply_mask(data: &mut [u8], mask: [u8; 4]) {
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= mask[i % 4];
    }
}

macro_rules! err {
    [$msg: expr] => {
        return Ok(Event::Error($msg))
    };
}
pub(crate) use err;

macro_rules! io_err {
    [$kind: ident, $msg: expr] => {
        return Err(std::io::Error::new(std::io::ErrorKind::$kind, $msg))
    };
}
pub(crate) use io_err;