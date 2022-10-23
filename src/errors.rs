use std::io::{Error, ErrorKind};

type DynErr = Box<dyn std::error::Error + Send + Sync>;

pub fn invalid_data(msg: impl Into<DynErr>) -> Error {
    Error::new(ErrorKind::InvalidData, msg)
}

pub fn invalid_input(msg: impl Into<DynErr>) -> Error {
    Error::new(ErrorKind::InvalidInput, msg)
}

pub fn conn_aborted() -> Error {
    Error::new(ErrorKind::ConnectionAborted, "The connection was aborted")
}

pub fn conn_closed() -> Error {
    Error::new(ErrorKind::NotConnected, "The connection was closed")
}
