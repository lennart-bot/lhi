//! Lightweight HTTP library

extern crate rustls;

mod errors;

pub mod server;

pub use errors::HttpError;
