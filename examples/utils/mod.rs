pub mod handshake;

pub trait OptionExt<T> {
    fn contains<U>(&self, x: U) -> bool
    where
        U: PartialEq<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn contains<U>(&self, x: U) -> bool
    where
        U: PartialEq<T>,
    {
        match self {
            Some(y) => x.eq(y),
            None => false,
        }
    }
}


pub fn connect() {
    
}