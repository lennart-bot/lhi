//! HTTP server

mod conn;
mod listener;
mod request;
mod response;

pub use conn::*;
pub use listener::*;
pub use request::*;
pub use response::*;

use kern::Fail;
use rustls::{ServerSession, Stream as RustlsStream};
use std::net::TcpStream;

pub type Stream<'a> = RustlsStream<'a, ServerSession, TcpStream>;
pub type Handler = fn(Result<HttpRequest, Fail>) -> Result<Vec<u8>, Fail>;

#[derive(Clone, Debug, Default)]
pub struct HttpOptions {
    pub max_header_size: usize,
    pub header_buffer: usize,
    pub max_body_size: usize,
    pub body_buffer: usize,
    pub body_read_attempts: usize,
}

impl HttpOptions {
    /// Create new HttpOptions with default values
    pub fn new() -> Self {
        Self {
            max_header_size: 8192,
            header_buffer: 8192,
            max_body_size: 10_485_760,
            body_buffer: 8192,
            body_read_attempts: 3,
        }
    }
}
