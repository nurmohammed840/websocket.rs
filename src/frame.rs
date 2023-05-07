#![doc(hidden)]

pub struct Frame<'a> {
    pub fin: bool,
    pub opcode: u8,
    pub data: &'a [u8],
}

impl<'a> Frame<'a> {
    #[inline]
    pub fn encode_without_mask(self) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(10 + self.data.len());
        unsafe {
            let dist = buf.as_mut_ptr();
            let head_len = self.encode_header_unchecked(dist, 0);
            std::ptr::copy_nonoverlapping(self.data.as_ptr(), dist.add(head_len), self.data.len());
            buf.set_len(head_len + self.data.len());
        }
        buf
    }

    #[inline]
    pub fn encode_with(self, mask: [u8; 4]) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(14 + self.data.len());
        unsafe {
            let dist = buf.as_mut_ptr();
            let head_len = self.encode_header_unchecked(dist, 0x80);

            let [a, b, c, d] = mask;
            dist.add(head_len).write(a);
            dist.add(head_len + 1).write(b);
            dist.add(head_len + 2).write(c);
            dist.add(head_len + 3).write(d);

            let dist = dist.add(head_len + 4);
            // TODO: Use SIMD wherever possible for best performance
            for i in 0..self.data.len() {
                dist.add(i)
                    .write(self.data.get_unchecked(i) ^ mask.get_unchecked(i & 3));
            }
            buf.set_len(head_len + 4 + self.data.len());
        }
        buf
    }

    /// # SEAFTY
    ///
    /// - `dist` must be valid for writes of 10 bytes.
    pub(crate) unsafe fn encode_header_unchecked(&self, dist: *mut u8, mask_bit: u8) -> usize {
        dist.write(((self.fin as u8) << 7) | self.opcode);
        if self.data.len() < 126 {
            dist.add(1).write(mask_bit | self.data.len() as u8);
            2
        } else if self.data.len() < 65536 {
            let [b2, b3] = (self.data.len() as u16).to_be_bytes();
            dist.add(1).write(mask_bit | 126);
            dist.add(2).write(b2);
            dist.add(3).write(b3);
            4
        } else {
            let [b2, b3, b4, b5, b6, b7, b8, b9] = (self.data.len() as u64).to_be_bytes();
            dist.add(1).write(mask_bit | 127);
            dist.add(2).write(b2);
            dist.add(3).write(b3);
            dist.add(4).write(b4);
            dist.add(5).write(b5);
            dist.add(6).write(b6);
            dist.add(7).write(b7);
            dist.add(8).write(b8);
            dist.add(9).write(b9);
            10
        }
    }
}

impl<'a> From<&'a str> for Frame<'a> {
    #[inline]
    fn from(string: &'a str) -> Self {
        Self {
            fin: true,
            opcode: 1,
            data: string.as_bytes(),
        }
    }
}

impl<'a> From<&'a [u8]> for Frame<'a> {
    #[inline]
    fn from(data: &'a [u8]) -> Self {
        Self {
            fin: true,
            opcode: 2,
            data,
        }
    }
}
