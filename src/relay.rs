use rusqlite::Connection;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::NeuroheatError;
use crate::heating_configuration::HeatingConfiguration;
use crate::repo;

const GPIO_PATH_PREFIX: &str = "/sys/class/gpio";

pub trait RelayController: std::fmt::Debug + Send + Sync {
    fn read_state(&self) -> Result<bool, NeuroheatError>;
    fn set_state(&self, state: bool) -> Result<(), NeuroheatError>;
    fn setup(&self) -> Result<(), NeuroheatError>;
}

#[derive(Debug)]
pub struct GPIOController {
    pin: u8,
}

impl GPIOController {
    pub fn new(pin: u8) -> Self {
        GPIOController { pin }
    }
}

impl RelayController for GPIOController {
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

    fn set_state(&self, state: bool) -> Result<(), NeuroheatError> {
        let path_str = format!("{}/gpio{}/value", GPIO_PATH_PREFIX, self.pin);
        let path = Path::new(&path_str);

        log::debug!("Setting GPIO pin {} to state {}", self.pin, state);

        let mut file = File::create(&path)?;
        let state_str = if state { "1" } else { "0" };
        file.write_all(state_str.as_bytes())?;

        Ok(())
    }

    fn setup(&self) -> Result<(), NeuroheatError> {
        let export_path = format!("{}/export", GPIO_PATH_PREFIX);
        let direction_path = format!("{}/gpio{}/direction", GPIO_PATH_PREFIX, self.pin);
        let gpio_path = format!("{}/gpio{}", GPIO_PATH_PREFIX, self.pin);

        if Path::new(&gpio_path).exists() {
            log::debug!("Pin {} is already exported", self.pin);
        } else {
            log::info!("Exporting pin {}", self.pin);
            let mut file = File::create(&export_path)?;
            file.write_all(self.pin.to_string().as_bytes())?;
        }

        let direction = "out";
        let current_direction = std::fs::read_to_string(&direction_path).unwrap_or_default();
        if current_direction.trim() == direction {
            log::debug!("Pin {} is already set as {}", self.pin, direction);
        } else {
            log::info!("Setting up pin {} as {}", self.pin, direction);
            let mut file = File::create(&direction_path)?;
            file.write_all(direction.as_bytes())?;
        }

        Ok(())
    }
}

pub fn setup_all_relays(config: &HeatingConfiguration) -> Result<(), NeuroheatError> {
    if let Some(stove_reader) = &config.stove_reader {
        stove_reader.setup()?;
    }

    for room in &config.rooms {
        if let Some(valve_reader) = &room.valve_reader {
            valve_reader.setup()?;
        }
    }

    Ok(())
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
