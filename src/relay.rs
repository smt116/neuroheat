use rusqlite::Connection;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::NeuroheatError;
use crate::heating_configuration::HeatingConfiguration;
use crate::repo;

const GPIO_PATH_PREFIX: &str = "/sys/class/gpio";

pub trait RelayStateReader: std::fmt::Debug + Send + Sync {
    fn read_state(&self) -> Result<bool, NeuroheatError>;
}

#[derive(Debug)]
pub struct GPIOReader {
    pin: u8,
}

impl GPIOReader {
    pub fn new(pin: u8) -> Self {
        GPIOReader { pin }
    }
}

impl RelayStateReader for GPIOReader {
    fn read_state(&self) -> Result<bool, NeuroheatError> {
        let path_str = format!("{}/gpio{}/value", GPIO_PATH_PREFIX, self.pin);
        let path = Path::new(&path_str);

        let file = File::open(&path)?;
        let mut reader = io::BufReader::new(file);
        let mut state_str = String::new();
        reader.read_line(&mut state_str)?;

        match state_str.trim() {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => {
                let err_msg = "Invalid state value".to_string();
                log::error!("{}", err_msg);
                Err(NeuroheatError::RelayError(err_msg))
            }
        }
    }
}

pub async fn read_relay_states(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<(), NeuroheatError> {
    // Read stove state
    if let Some(stove_reader) = &config.stove_reader {
        match stove_reader.read_state() {
            Ok(state) => {
                log::info!("Stove State: {}", state);
                if let Err(e) = repo::store_state(&conn, "stove", state) {
                    log::error!("Failed to store stove state: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Error reading stove state: {}", e);
            }
        }
    }

    for room in &config.rooms {
        if let Some(valve_reader) = &room.valve_reader {
            match valve_reader.read_state() {
                Ok(state) => {
                    log::info!("Room: {}, Valve State: {}", room.name, state);
                    if let Err(e) = repo::store_state(&conn, &room.key, state) {
                        log::error!("Failed to store valve state for room {}: {}", room.name, e);
                    }
                }
                Err(e) => {
                    log::warn!("Error reading valve state for {}: {}", room.name, e);
                }
            }
        } else {
            log::warn!("No valve reader found for room: {}", room.name);
        }
    }

    Ok(())
}
