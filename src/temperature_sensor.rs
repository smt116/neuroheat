use rusqlite::Connection;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::NeuroheatError;
use crate::heating_configuration::HeatingConfiguration;
use crate::repo;

const W1_PATH_PREFIX: &str = "/sys/devices/w1_bus_master1/";

pub struct DS18B20 {
    id: String,
    file_path: String,
}

impl DS18B20 {
    pub fn new(id: String) -> Self {
        DS18B20 {
            id: id.clone(),
            file_path: format!("{}/{}/w1_slave", W1_PATH_PREFIX, id),
        }
    }
}

impl std::fmt::Debug for DS18B20 {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("DS18B20")
            .field("id", &self.id)
            .field("file_path", &self.file_path)
            .finish()
    }
}

pub trait TemperatureSensor: std::fmt::Debug + Send + Sync {
    fn read(&self) -> Result<f32, NeuroheatError>;
}

impl TemperatureSensor for DS18B20 {
    fn read(&self) -> Result<f32, NeuroheatError> {
        let path = Path::new(&self.file_path);

        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);

        let mut lines = reader.lines();
        let first_line = lines.next().ok_or_else(|| {
            let err_msg = "No first line".to_string();
            log::error!("{}", err_msg);
            NeuroheatError::SensorError(err_msg)
        })??;

        if !first_line.ends_with("YES") {
            let err_msg = "CRC check failed".to_string();
            log::error!("{}", err_msg);
            return Err(NeuroheatError::SensorError(err_msg));
        }

        let second_line = lines.next().ok_or_else(|| {
            let err_msg = "No second line".to_string();
            log::error!("{}", err_msg);
            NeuroheatError::SensorError(err_msg)
        })??;

        if let Some(pos) = second_line.find("t=") {
            let temp_str = &second_line[pos + 2..];
            let temp_millidegrees: i32 = temp_str.parse().map_err(|_| {
                let err_msg = "Failed to parse temperature".to_string();
                log::error!("{}", err_msg);
                NeuroheatError::SensorError(err_msg)
            })?;
            let temperature = temp_millidegrees as f32 / 1000.0;

            if temperature < 0.0 || temperature > 50.0 {
                let err_msg = format!("Temperature out of range: {:.1}°C", temperature);
                log::error!("{}", err_msg);
                Err(NeuroheatError::SensorError(err_msg))
            } else {
                Ok(temperature)
            }
        } else {
            let err_msg = "Temperature data not found".to_string();
            log::error!("{}", err_msg);
            Err(NeuroheatError::SensorError(err_msg))
        }
    }
}

pub async fn read_temperatures(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<(), NeuroheatError> {
    if let Some(pipe_sensor) = &config.pipe_sensor {
        match pipe_sensor.read() {
            Ok(temp) => {
                log::info!("Pipe Temperature: {:.1}°C", temp);
                if let Err(e) = repo::store_temperature(&conn, "pipe", temp, None) {
                    log::error!("Failed to store pipe temperature: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Error reading pipe temperature: {}", e);
            }
        }
    }

    for room in &config.rooms {
        if let Some(sensor) = &room.sensor {
            match sensor.read() {
                Ok(temp) => {
                    let expected_temp = room.get_expected_temperature();
                    match expected_temp {
                        Some(expected) => {
                            log::info!(
                                "Room: {}, Temperature: {:.1}°C, Expected Temperature: {:.1}°C",
                                room.name,
                                temp,
                                expected
                            );
                        }
                        None => {
                            log::info!("Room: {}, Temperature: {:.1}°C", room.name, temp);
                        }
                    }
                    if let Err(e) = repo::store_temperature(&conn, &room.key, temp, expected_temp) {
                        log::error!("Failed to store temperature for room {}: {}", room.name, e);
                    }
                }
                Err(e) => {
                    log::warn!("Error reading temperature for {}: {}", room.name, e);
                }
            }
        } else {
            log::warn!("No sensor found for room: {}", room.name);
        }
    }

    Ok(())
}
