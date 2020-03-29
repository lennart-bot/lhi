//! Lightweight HTTP library

pub mod common;
pub mod server;

use common::CARGO_TOML;
use kern::{init_version, version as get_version};

/// Get lhi version string
pub fn version() -> &'static str {
    match get_version() {
        "" => init_version(CARGO_TOML),
        version => version,
    }
}

// TODO add tests
