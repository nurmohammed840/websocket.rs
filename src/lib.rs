mod errors;
mod frame;
mod mask;
mod utils;
mod ws;

pub mod handshake;
pub mod http;
pub use ws::*;
pub use frame::*;

use errors::*;
use mask::*;
use utils::*;

use std::io::Result;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Text,
    Binary,
}
