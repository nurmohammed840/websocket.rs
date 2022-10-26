use super::*;

pub async fn read_buf<const N: usize>(stream: &mut BufReader<TcpStream>) -> Result<[u8; N]> {
    let mut buf = [0; N];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

#[inline]
pub async fn read_bytes(
    stream: &mut BufReader<TcpStream>,
    len: usize,
    cb: impl FnOnce(&[u8]),
) -> Result<usize> {
    let bytes = stream.fill_buf().await?;
    if bytes.is_empty() {
        return err(ErrorKind::ConnectionAborted, "The connection was aborted");
    }
    let amt = bytes.len().min(len);
    cb(unsafe { bytes.get_unchecked(..amt) });
    stream.consume(amt);
    Ok(amt)
}
