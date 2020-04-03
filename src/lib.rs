//! Lightweight HTTP library

pub mod common;
pub mod server;

mod cargo_name;

use cargo_name::{init_name, name as get_name};
use common::CARGO_TOML;
use kern::{init_version, version as get_version};

/// Get lhi version string
pub fn version() -> &'static str {
    match get_version() {
        "" => init_version(CARGO_TOML),
        version => version,
    }
}

/// Get lhi name string
pub fn name() -> &'static str {
    match get_name() {
        "" => init_name(CARGO_TOML),
        name => name,
    }
}

// TODO add tests
