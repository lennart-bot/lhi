//! HTTP server

mod conn;
mod listener;
mod request;
mod response;

pub use conn::*;
pub use listener::*;
pub use request::*;
pub use response::*;

use crate::HttpError;
use rustls::{ServerSession, Stream as RustlsStream};
use std::net::TcpStream;

pub type Stream<'a> = RustlsStream<'a, ServerSession, TcpStream>;
pub type Handler = fn(Result<HttpRequest, HttpError>) -> Result<Vec<u8>, HttpError>;
