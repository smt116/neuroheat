use std::fmt;
use std::io;

#[derive(Debug)]
pub enum NeuroheatError {
    ConfigurationError(String),
    ControllerError(String),
    DatabaseError(String),
    RelayError(String),
    SensorError(String),
}

impl fmt::Display for NeuroheatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeuroheatError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            NeuroheatError::ControllerError(msg) => write!(f, "Database error: {}", msg),
            NeuroheatError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            NeuroheatError::RelayError(msg) => write!(f, "Relay error: {}", msg),
            NeuroheatError::SensorError(msg) => write!(f, "Sensor error: {}", msg),
        }
    }
}

impl std::error::Error for NeuroheatError {}

impl From<io::Error> for NeuroheatError {
    fn from(error: io::Error) -> Self {
        NeuroheatError::SensorError(error.to_string())
    }
}

impl From<rusqlite::Error> for NeuroheatError {
    fn from(error: rusqlite::Error) -> Self {
        NeuroheatError::DatabaseError(error.to_string())
    }
}
