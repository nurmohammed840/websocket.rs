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

// ----------------------------------------------------------------------

struct XorShift128Plus {
    x: u64,
    y: u64,
}

impl XorShift128Plus {
    fn next(&mut self) -> u64 {
        let Self { x, y } = *self;
        let t = x ^ (x << 23);
        self.x = y;
        self.y = t ^ y ^ (t >> 17) ^ (y >> 26);
        self.y.wrapping_add(y)
    }
}

static mut RNG: XorShift128Plus = XorShift128Plus {
    x: 0x_C01D,
    y: 0x_C0F1,
};

pub fn set_random_seed() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let x = now.as_secs();
    let y = now.subsec_nanos() as u64;
    unsafe { RNG = XorShift128Plus { x, y } }
}

pub fn rand_num() -> u64 {
    unsafe { RNG.next() }
}

#[allow(dead_code)]
pub fn rand_u128() -> u128 {
    unsafe {
        let high = RNG.next() as u128;
        let low = RNG.next() as u128;
        (high << 64) | low
    }
}
