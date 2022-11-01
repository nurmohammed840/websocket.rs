/// Rsv are used for extensions.
#[derive(Default, Eq, PartialEq, Clone, Copy)]
pub struct Rsv(pub u8);

impl Rsv {
    /// The first bit of the RSV field.
    pub fn rsv1(&self) -> bool {
        self.0 & 0b100_0000 != 0
    }

    /// The second bit of the RSV field.
    pub fn rsv2(&self) -> bool {
        self.0 & 0b10_0000 != 0
    }

    /// The third bit of the RSV field.
    pub fn rsv3(&self) -> bool {
        self.0 & 0b1_0000 != 0
    }
}

impl std::fmt::Debug for Rsv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#b}", (self.0 >> 4) & 0b111)
    }
}
