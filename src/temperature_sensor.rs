use rusqlite::Connection;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::{Arc, Mutex};

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
    fn read(&self) -> Result<f32, Box<dyn Error + Send + Sync>>;
}

impl TemperatureSensor for DS18B20 {
    fn read(&self) -> Result<f32, Box<dyn Error + Send + Sync>> {
        let path = Path::new(&self.file_path);

        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);

        let mut lines = reader.lines();
        let first_line = lines
            .next()
            .ok_or(io::Error::new(io::ErrorKind::Other, "No first line"))??;

        if !first_line.ends_with("YES") {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "CRC check failed",
            )));
        }

        let second_line = lines
            .next()
            .ok_or(io::Error::new(io::ErrorKind::Other, "No second line"))??;

        if let Some(pos) = second_line.find("t=") {
            let temp_str = &second_line[pos + 2..];
            let temp_millidegrees: i32 = temp_str
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to parse temperature"))?;
            let temperature = temp_millidegrees as f32 / 1000.0;

            if temperature < 0.0 || temperature > 50.0 {
                Err(format!("Temperature out of range: {:.1}°C", temperature).into())
            } else {
                Ok(temperature)
            }
        } else {
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Temperature data not found",
            )))
        }
    }
}

pub async fn read_temperatures(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(pipe_sensor) = &config.pipe_sensor {
        match pipe_sensor.read() {
            Ok(temp) => {
                log::info!("Pipe Temperature: {:.1}°C", temp);
                repo::store_temperature(&conn, "pipe", temp)?;
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
                    log::info!("Room: {}, Temperature: {:.1}°C", room.name, temp);
                    repo::store_temperature(&conn, &room.key, temp)?;
                }
                Err(e) => {
                    log::warn!("Error reading temperature for {}: {}", room.name, e);
                }
            }
        }
    }

    Ok(())
}
