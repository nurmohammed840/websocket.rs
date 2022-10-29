#![doc = include_str!("../README.md")]

mod errors;
mod frame;
mod mask;
mod utils;
mod ws;

pub mod handshake;
pub mod http;
pub use frame::*;
pub use ws::*;

use errors::*;
use mask::*;
use utils::*;

use std::io::Result;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Text,
    Binary,
}

#[derive(Debug, Clone)]
pub enum Event<'a> {
    Ping(&'a [u8]),
    Pong(&'a [u8]),
}

impl Event<'_> {
    #[inline]
    pub fn data(&self) -> &[u8] {
        match self {
            Event::Ping(data) => data,
            Event::Pong(data) => data,
        }
    }
}

impl std::fmt::Display for Event<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::str::from_utf8(self.data()).unwrap_or(""))
    }
}
