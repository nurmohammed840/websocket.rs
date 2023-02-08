use std::io::Error;
pub use std::io::ErrorKind;

type DynErr = Box<dyn std::error::Error + Send + Sync>;

#[inline]
pub fn err<T>(kind: ErrorKind, msg: impl Into<DynErr>) -> std::io::Result<T> {
    Err(Error::new(kind, msg))
}

#[inline]
pub fn proto_err<T>(msg: impl Into<DynErr>) -> std::io::Result<T> {
    err(ErrorKind::InvalidData, msg)
}

#[inline]
pub fn invalid_data(msg: impl Into<DynErr>) -> Error {
    Error::new(ErrorKind::InvalidData, msg)
}
