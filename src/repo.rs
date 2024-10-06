use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db;
use crate::error::NeuroheatError;

pub fn get_latest_temperature(
    conn: &Arc<Mutex<Connection>>,
    key: &str,
) -> Result<HashMap<&'static str, String>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.query_row(
            "SELECT
               temperatures.key,
               COALESCE(labels.label, temperatures.key),
               temperatures.timestamp,
               temperatures.temperature,
               temperatures.expected_temperature
             FROM temperatures
             LEFT JOIN labels ON labels.key = temperatures.key
             WHERE temperatures.key = ?
             ORDER BY temperatures.timestamp DESC LIMIT 1",
            params![key],
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
        let err_msg = format!("Failed to get latest temperature for key {}: {}", key, e);
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
               temperatures.key,
               COALESCE(labels.label, temperatures.key),
               temperatures.timestamp,
               temperatures.temperature,
               temperatures.expected_temperature
             FROM labels
             LEFT JOIN temperatures ON labels.key = temperatures.key
             AND temperatures.timestamp = (
                 SELECT MAX(timestamp)
                 FROM temperatures
                 WHERE key = labels.key
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
    key: &str,
    temperature: f32,
    expected_temperature: Option<f32>,
) -> Result<(), NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.execute(
            "INSERT INTO temperatures (key, temperature, expected_temperature) VALUES (?1, ?2, ?3)",
            params![key, temperature, expected_temperature],
        )
        .map(|_| ())
    })
    .map_err(|e| {
        let err_msg = format!("Failed to store temperature for key {}: {}", key, e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}
