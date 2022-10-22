mod errors;
mod mask;
mod utils;
mod ws;

pub mod http;
pub mod frame;
pub mod handshake;
pub use ws::*;

use errors::*;
use mask::*;
use utils::*;

use std::io::Result;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, ToSocketAddrs},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Text,
    Binary,
}
