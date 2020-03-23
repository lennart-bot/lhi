//! Collection of errors

use std::error::Error;
use std::fmt;

/// HTTP errors for library (also custom)
#[derive(Clone, Debug)]
pub struct HttpError(String);

// Error implementation
impl HttpError {
    /// Create Error from any Display
    pub fn new<E>(err: E) -> Self
    where
        E: fmt::Display,
    {
        Self(err.to_string())
    }

    /// Create Result with Error from any Display
    pub fn from<T, E>(err: E) -> Result<T, Self>
    where
        E: fmt::Display,
    {
        Err(Self::new(err))
    }
}

/// Display implementation for HttpError
impl fmt::Display for HttpError {
    // fmt implementation
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Error implementation for HttpError
impl Error for HttpError {}
