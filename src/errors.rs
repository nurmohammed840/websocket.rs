// use super::{CloseEvent, Event};
// use std::{error, fmt};

// macro_rules! err {
//     [$kind: ident, $err: expr] => {
//         return Err(std::io::Error::new(std::io::ErrorKind::$kind, $err))
//     };
//     [$err: expr] => {
//         return Err(std::io::Error::new(std::io::ErrorKind::Other, $err))
//     };
// }
// pub(crate) use err;

// impl error::Error for CloseEvent {}
// impl fmt::Display for CloseEvent {
//     #[inline]
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             CloseEvent::Error(msg) => f.write_str(msg),
//             CloseEvent::Close { reason, .. } => f.write_str(reason),
//         }
//     }
// }

// impl<Data: AsRef<[u8]>> fmt::Display for Event<Data> {
//     #[inline]
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "{}",
//             std::str::from_utf8(self.data().as_ref()).unwrap_or("")
//         )
//     }
// }
