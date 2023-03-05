use super::{CloseEvent, Event};
use std::{error, fmt};

macro_rules! err {
    [$kind: ident, $err: expr] => {
        return Err(std::io::Error::new(std::io::ErrorKind::$kind, $err))
    };
    [$err: expr] => {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, $err))
    };
}
pub (crate) use err;

impl error::Error for CloseEvent {}
impl fmt::Display for CloseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloseEvent::Error(msg) => f.write_str(&msg),
            CloseEvent::Close { reason, .. } => f.write_str(&reason),
        }
    }
}

impl fmt::Display for Event<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", std::str::from_utf8(self.data()).unwrap_or(""))
    }
}