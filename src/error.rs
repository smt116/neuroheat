use std::fmt;
use std::io;

#[derive(Debug)]
pub enum NeuroheatError {
    DatabaseError(String),
    SensorError(String),
    ConfigurationError(String),
}

impl fmt::Display for NeuroheatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeuroheatError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            NeuroheatError::SensorError(msg) => write!(f, "Sensor error: {}", msg),
            NeuroheatError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
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
