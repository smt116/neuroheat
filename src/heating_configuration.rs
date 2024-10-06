use crate::temperature_sensor::{TemperatureSensor, DS18B20};
use serde::Deserialize;
use std::error::Error;
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
}

impl HeatingConfiguration {
    /// Reads the heating configuration from a JSON file.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut config: HeatingConfiguration = serde_json::from_reader(reader)?;

        // Initialize sensors
        for room in &mut config.rooms {
            room.sensor = Some(Arc::new(DS18B20::new(room.sensor_id.clone())));
        }
        config.pipe_sensor = Some(Arc::new(DS18B20::new(config.pipe_sensor_id.clone())));

        Ok(config)
    }
}
