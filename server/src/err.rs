use super::*;

pub fn conn_aborted<T>() -> Result<T> {
    Err(Error::new(ErrorKind::ConnectionAborted, "The connection was aborted"))
}


pub fn proto<T>(msg: &str) -> Result<T> {
    Err(Error::new(ErrorKind::InvalidData, format!("Protocol error: {msg}")))
}
