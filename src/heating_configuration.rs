use crate::error::NeuroheatError;
use crate::relay::{GPIOController, RelayController};
use crate::temperature_sensor::{TemperatureSensor, DS18B20};

use chrono::{Local, Timelike};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

/// Represents a room in the house.
#[derive(Debug, Deserialize)]
pub struct Room {
    /// A unique key identifying the room.
    pub key: String,
    /// The label of the room.
    pub name: String,
    /// The sensor ID associated with the room.
    pub sensor_id: String,
    /// The temperature sensor associated with the room.
    #[serde(skip)]
    pub sensor: Option<Arc<dyn TemperatureSensor>>,
    /// The GPIO pin controlling the valve for the floor heating for the room.
    pub valve_pin: u8,
    /// The area of the room in square meters.
    pub area: f32,
    /// The temperature schedule for the room.
    pub temperature_schedule: Vec<TemperatureSchedule>,
    /// The relay reader for the valve.
    #[serde(skip)]
    pub valve_reader: Option<Arc<dyn RelayController>>,
}

/// Represents a temperature schedule for a room.
#[derive(Debug, Deserialize)]
pub struct TemperatureSchedule {
    /// The start hour of the schedule (0-23).
    pub start_hour: u8,
    /// The end hour of the schedule (0-23).
    pub end_hour: u8,
    /// The target temperature during the schedule period.
    pub temperature: f32,
}

/// Represents the heating configuration for the entire system.
#[derive(Debug, Deserialize)]
pub struct HeatingConfiguration {
    /// A list of rooms in the house.
    pub rooms: Vec<Room>,
    /// The GPIO pin controlling the stove.
    pub stove_pin: u8,
    /// The sensor ID for the heating pipe.
    pub pipe_sensor_id: String,
    /// The temperature sensor for the heating pipe.
    #[serde(skip)]
    pub pipe_sensor: Option<Arc<dyn TemperatureSensor>>,
    /// The relay reader for the stove.
    #[serde(skip)]
    pub stove_reader: Option<Arc<dyn RelayController>>,
}

impl HeatingConfiguration {
    /// Reads the heating configuration from a JSON file.
    pub fn from_file(path: &str) -> Result<Self, NeuroheatError> {
        let file = File::open(path).map_err(|e| {
            let err_msg = format!("Failed to open configuration file {}: {}", path, e);
            log::error!("{}", err_msg);
            NeuroheatError::ConfigurationError(err_msg)
        })?;
        let reader = BufReader::new(file);
        let mut config: HeatingConfiguration = serde_json::from_reader(reader).map_err(|e| {
            let err_msg = format!("Failed to parse configuration file {}: {}", path, e);
            log::error!("{}", err_msg);
            NeuroheatError::ConfigurationError(err_msg)
        })?;

        for room in &mut config.rooms {
            room.sensor = Some(Arc::new(DS18B20::new(room.sensor_id.clone())));
            room.valve_reader = Some(Arc::new(GPIOController::new(room.valve_pin)));
        }
        config.pipe_sensor = Some(Arc::new(DS18B20::new(config.pipe_sensor_id.clone())));
        config.stove_reader = Some(Arc::new(GPIOController::new(config.stove_pin)));

        Ok(config)
    }
}

impl Room {
    pub fn get_expected_temperature(&self) -> Option<f32> {
        let current_hour = Local::now().hour() as u8;

        self.temperature_schedule
            .iter()
            .find(|schedule| {
                current_hour >= schedule.start_hour && current_hour < schedule.end_hour
            })
            .map(|schedule| schedule.temperature)
    }
}
