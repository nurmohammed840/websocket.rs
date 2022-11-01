pub trait Frame {
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>);
}

impl<T: Frame + ?Sized> Frame for &T {
    #[inline]
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        T::encode::<SIDE>(self, writer)
    }
}

impl<T: Frame + ?Sized> Frame for Box<T> {
    #[inline]
    fn encode<const SIDE: bool>(&self, writer: &mut Vec<u8>) {
        T::encode::<SIDE>(self, writer)
    }
}

impl Frame for str {
    #[inline]
    fn encode<const MASK: bool>(&self, writer: &mut Vec<u8>) {
        encode::<MASK>(writer, true, 1, self.as_bytes());
    }
}

impl Frame for [u8] {
    #[inline]
    fn encode<const MASK: bool>(&self, writer: &mut Vec<u8>) {
        encode::<MASK>(writer, true, 2, self);
    }
}

impl<const N: usize> Frame for [u8; N] {
    #[inline]
    fn encode<const MASK: bool>(&self, writer: &mut Vec<u8>) {
        encode::<MASK>(writer, true, 2, self);
    }
}

pub fn encode<const MASK: bool>(
    writer: &mut Vec<u8>,
    fin: bool,
    opcode: impl Into<u8>,
    data: &[u8],
) {
    let data_len = data.len();
    writer.reserve(if MASK { 14 } else { 10 } + data_len);
    unsafe {
        let filled = writer.len();
        let start = writer.as_mut_ptr().add(filled);

        let mask_bit = if MASK { 0x80 } else { 0 };

        start.write(((fin as u8) << 7) | opcode.into());
        let len = if data_len < 126 {
            start.add(1).write(mask_bit | data_len as u8);
            2
        } else if data_len < 65536 {
            let [b2, b3] = (data_len as u16).to_be_bytes();
            start.add(1).write(mask_bit | 126);
            start.add(2).write(b2);
            start.add(3).write(b3);
            4
        } else {
            let [b2, b3, b4, b5, b6, b7, b8, b9] = (data_len as u64).to_be_bytes();
            start.add(1).write(mask_bit | 127);
            start.add(2).write(b2);
            start.add(3).write(b3);
            start.add(4).write(b4);
            start.add(5).write(b5);
            start.add(6).write(b6);
            start.add(7).write(b7);
            start.add(8).write(b8);
            start.add(9).write(b9);
            10
        };

        let header_len = if !MASK {
            std::ptr::copy_nonoverlapping(data.as_ptr(), start.add(len), data_len);
            len
        } else {
            let mask = fastrand::u32(..).to_ne_bytes();
            let [a, b, c, d] = mask;
            start.add(len).write(a);
            start.add(len + 1).write(b);
            start.add(len + 2).write(c);
            start.add(len + 3).write(d);

            let dist = start.add(len + 4);
            for (index, byte) in data.iter().enumerate() {
                dist.add(index).write(byte ^ mask.get_unchecked(index % 4));
            }
            len + 4
        };
        writer.set_len(filled + header_len + data_len);
    }
}
