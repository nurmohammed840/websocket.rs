pub struct RandMask;

pub trait RandKeys {
    fn keys() -> [u8; 4];
}

impl RandKeys for RandMask {
    fn keys() -> [u8; 4] {
        fastrand::u32(..).to_ne_bytes()
    }
}

pub struct Mask {
    index: usize,
    pub keys: [u8; 4],
}

impl From<[u8; 4]> for Mask {
    fn from(keys: [u8; 4]) -> Self {
        Self { index: 0, keys }
    }
}

impl Iterator for Mask {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys[self.index % 4];
        self.index += 1;
        Some(key)
    }
}
