use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db;
use crate::error::NeuroheatError;
use crate::heating_configuration::HeatingConfiguration;

pub fn init(
    conn: &Arc<Mutex<Connection>>,
    config: &HeatingConfiguration,
) -> Result<(), NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS labels (
              key TEXT PRIMARY KEY,
              label TEXT NOT NULL
          )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS temperatures (
              id INTEGER PRIMARY KEY,
              room_key TEXT NOT NULL,
              temperature REAL NOT NULL,
              expected_temperature REAL,
              timestamp TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
              FOREIGN KEY(room_key) REFERENCES labels(key)
          )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS temperatures_room_key_timestamp_idx ON temperatures (room_key, timestamp)",
            [],
        )?;

        for room in &config.rooms {
            conn.execute(
                "INSERT OR IGNORE INTO labels (key, label) VALUES (?1, ?2)",
                params![room.key, room.name],
            )?;
        }

        // Insert a value for the pipe as "Heating Pipe"
        conn.execute(
            "INSERT OR IGNORE INTO labels (key, label) VALUES (?1, ?2)",
            params!["pipe", "Heating Pipe"],
        )?;

        Ok(())
    })
    .map_err(|e| {
        let err_msg = format!("Failed to initialize database: {}", e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}

pub fn get_latest_temperature(
    conn: &Arc<Mutex<Connection>>,
    room_key: &str,
) -> Result<HashMap<&'static str, String>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.query_row(
            "SELECT
               temperatures.room_key,
               COALESCE(labels.label, temperatures.room_key),
               temperatures.timestamp,
               temperatures.temperature,
               temperatures.expected_temperature
             FROM temperatures
             LEFT JOIN labels ON labels.key = temperatures.room_key
             WHERE temperatures.room_key = ?
             ORDER BY temperatures.timestamp DESC LIMIT 1",
            params![room_key],
            |row| {
                let mut result = HashMap::from([
                    ("key", row.get::<_, String>(0)?),
                    ("label", row.get::<_, String>(1)?),
                    ("timestamp", row.get::<_, String>(2)?),
                    ("temperature", row.get::<_, f32>(3)?.to_string()),
                ]);
                if let Some(expected_temp) = row.get::<_, Option<f32>>(4)? {
                    result.insert("expected_temperature", expected_temp.to_string());
                }
                Ok(result)
            },
        )
    })
    .map_err(|e| {
        let err_msg = format!(
            "Failed to get latest temperature for room {}: {}",
            room_key, e
        );
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}

pub fn get_latest_temperatures(
    conn: &Arc<Mutex<Connection>>,
) -> Result<HashMap<String, HashMap<&'static str, String>>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        let mut stmt = conn.prepare(
            "SELECT
               temperatures.room_key,
               COALESCE(labels.label, temperatures.room_key),
               temperatures.timestamp,
               temperatures.temperature,
               temperatures.expected_temperature
             FROM labels
             LEFT JOIN temperatures ON labels.key = temperatures.room_key
             AND temperatures.timestamp = (
                 SELECT MAX(timestamp)
                 FROM temperatures
                 WHERE room_key = labels.key
             )",
        )?;

        let result = stmt
            .query_map([], |row| {
                let key = row.get::<_, String>(0)?;
                let label = row.get::<_, String>(1)?;
                let timestamp = row.get::<_, String>(2)?;
                let temperature = row.get::<_, f32>(3)?.to_string();
                let mut map = HashMap::from([
                    ("label", label),
                    ("timestamp", timestamp),
                    ("temperature", temperature),
                ]);
                if let Some(expected_temp) = row.get::<_, Option<f32>>(4)? {
                    map.insert("expected_temperature", expected_temp.to_string());
                }
                Ok((key, map))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(result)
    })
    .map_err(|e| {
        let err_msg = format!("Failed to get latest temperatures: {}", e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}

pub fn store_temperature(
    conn: &Arc<Mutex<Connection>>,
    room_key: &str,
    temperature: f32,
    expected_temperature: Option<f32>,
) -> Result<(), NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.execute(
            "INSERT INTO temperatures (room_key, temperature, expected_temperature) VALUES (?1, ?2, ?3)",
            params![room_key, temperature, expected_temperature],
        ).map(|_| ())
    })
    .map_err(|e| {
        let err_msg = format!("Failed to store temperature for room {}: {}", room_key, e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}
