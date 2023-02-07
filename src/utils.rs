use super::*;

pub async fn read_buf<const N: usize, R>(stream: &mut R) -> Result<[u8; N]>
where
    R: Unpin + AsyncRead,
{
    let mut buf = [0; N];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn read_bytes<R>(stream: &mut R, len: usize, cb: impl FnOnce(&[u8])) -> Result<usize>
where
    R: Unpin + AsyncBufRead,
{
    let bytes = stream.fill_buf().await?;
    if bytes.is_empty() {
        return err(ErrorKind::ConnectionAborted, "The connection was aborted");
    }
    let amt = bytes.len().min(len);
    cb(unsafe { bytes.get_unchecked(..amt) });
    stream.consume(amt);
    Ok(amt)
}
