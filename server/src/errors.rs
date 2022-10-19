use std::io::{Error, ErrorKind, Result};

type DynErr = Box<dyn std::error::Error + Send + Sync>;

pub fn err<T>(msg: impl Into<DynErr>) -> Result<T> {
    Err(Error::new(ErrorKind::InvalidData, msg))
}

// pub fn conn_aborted<T>() -> Result<T> {
//     Err(Error::new(ErrorKind::ConnectionAborted, "The connection was aborted"))
// }

pub fn conn_closed<T>() -> Result<T> {
    Err(Error::new(ErrorKind::NotConnected, "The connection was closed"))
}
