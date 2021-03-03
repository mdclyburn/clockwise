//! Testing errors

use std::error;
use std::fmt;
use std::fmt::Display;

use rppal::gpio;

use crate::io;

/// Test-related error.
#[derive(Debug)]
pub enum Error {
    /// Testbed to device I/O error
    IO(io::Error),
    GPIO(gpio::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::IO(ref e) => Some(e),
            Error::GPIO(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<gpio::Error> for Error {
    fn from(e: gpio::Error) -> Self {
        Error::GPIO(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IO(ref e) => write!(f, "I/O error: {}", e),
            Error::GPIO(ref e) => write!(f, "GPIO error while testing: {}", e),
        }
    }
}