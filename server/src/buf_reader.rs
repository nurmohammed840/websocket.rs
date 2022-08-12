use super::*;

/// Same as `std::io::BufReader` + Some costom methods
pub struct BufferReader<R> {
    pub inner: R,
    buf: Box<[u8]>,
    pos: usize,
    cap: usize,
}

impl<R: Read> BufferReader<R> {
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            inner,
            buf: vec![0; capacity].into(),
            pos: 0,
            cap: 0,
        }
    }

    #[inline]
    pub fn buffer(&self) -> &[u8] {
        // Safety (checked): self.pos <= self.cap && self.cap <= self.buf.len()
        unsafe { self.buf.get_unchecked(self.pos..self.cap) }
    }

    #[inline]
    pub fn fill_buf(&mut self) -> Result<&[u8]> {
        // If we've reached the end of our internal buffer then we need to fetch some more data from the underlying reader.
        // Branch using `>=` instead of the more correct `==` to tell the compiler that the pos..cap slice is always valid.
        if self.pos >= self.cap {
            debug_assert!(self.pos == self.cap);
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(self.buffer())
    }

    #[inline]
    pub fn consume(&mut self, amt: usize) {
        self.pos = std::cmp::min(self.pos + amt, self.cap);
    }

    // ---------------------------- Costom methods ----------------------------------

    /// Ensure that internal buffer has atleast `nbytes` of data (in length) available.
    pub fn ensure_data(&mut self, nbytes: usize) -> Result<()> {
        let nbytes = self.buf.len().min(nbytes);
        let len = self.cap - self.pos;
        if len >= nbytes {
            return Ok(());
        }
        unsafe {
            let ptr = self.buf.as_mut_ptr();
            std::ptr::copy(ptr.add(self.pos), ptr, len);
            self.pos = 0;
            self.cap = len;
        }
        let mut unfilled = &mut self.buf[len..];
        loop {
            let n = match self.inner.read(unfilled) {
                Ok(0) => return err::conn_aborted(),
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            self.cap += n;
            // since `self.pos` is 0, therefore `self.cap` is filled data length.
            if self.cap >= nbytes {
                break Ok(());
            }
            unfilled = &mut unfilled[n..];
        }
    }
}

mod test {
    use super::*;
    struct Stream(u8);
    impl Read for Stream {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            for v in buf.iter_mut() {
                self.0 += 1;
                *v = self.0;
            }
            Ok(buf.len())
        }
    }

    #[test]
    fn test_ensure_data() -> Result<()> {
        let mut reader = BufferReader::with_capacity(4, Stream(0));
        assert!(reader.buffer().is_empty());

        reader.ensure_data(4)?;
        assert_eq!(reader.buffer(), [1, 2, 3, 4]);

        reader.consume(2);

        reader.ensure_data(2)?;
        assert_eq!(reader.buffer(), [3, 4]);

        reader.ensure_data(3)?;
        assert_eq!(reader.buffer(), [3, 4, 5, 6]);
        Ok(())
    }
}
