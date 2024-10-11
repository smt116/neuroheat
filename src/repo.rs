use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db;
use crate::error::NeuroheatError;

pub fn get_current_state(
    conn: &Arc<Mutex<Connection>>,
) -> Result<HashMap<String, HashMap<&'static str, String>>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        let mut stmt = conn.prepare(
            r#"
            SELECT
              temperatures.key,
              COALESCE(labels.label, temperatures.key),
              temperatures.timestamp,
              temperatures.temperature,
              temperatures.expected_temperature,
              states.state
            FROM labels
            LEFT JOIN temperatures ON labels.key = temperatures.key
            AND temperatures.timestamp = (
              SELECT MAX(timestamp)
              FROM temperatures
              WHERE key = labels.key
            )
            LEFT JOIN states ON labels.key = states.key
            AND states.timestamp = (
              SELECT MAX(timestamp)
              FROM states
              WHERE key = states.key
            )
            WHERE temperatures.key IS NOT NULL
            "#,
        )?;

        let mut result = stmt
            .query_map([], |row| {
                let key = row.get::<_, String>(0)?;
                let label = row.get::<_, String>(1)?;
                let timestamp_str = row.get::<_, String>(2)?;
                let timestamp = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc().to_string())
                    .unwrap_or_else(|_| timestamp_str);
                let temperature = row.get::<_, f32>(3)?.to_string();
                let mut map = HashMap::from([
                    ("label", label),
                    ("timestamp", timestamp),
                    ("temperature", temperature),
                ]);
                if let Some(expected_temp) = row.get::<_, Option<f32>>(4)? {
                    map.insert("expected_temperature", expected_temp.to_string());
                }
                if let Some(state) = row.get::<_, Option<i32>>(5)? {
                    map.insert("heating_enabled", (state != 0).to_string());
                }
                Ok((key, map))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;

        let stove_state = conn.query_row(
            r#"
            SELECT
              states.key,
              COALESCE(labels.label, states.key),
              states.timestamp,
              states.state
            FROM states
            LEFT JOIN labels ON labels.key = states.key
            WHERE states.key = 'stove'
            ORDER BY states.timestamp DESC
            LIMIT 1
            "#,
            [],
            |row| {
                let key = row.get::<_, String>(0)?;
                let label = row.get::<_, String>(1)?;
                let timestamp_str = row.get::<_, String>(2)?;
                let timestamp = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc().to_string())
                    .unwrap_or_else(|_| timestamp_str);
                let state = (row.get::<_, i32>(3)? != 0).to_string();
                let map = HashMap::from([
                    ("label", label),
                    ("timestamp", timestamp),
                    ("heating_enabled", state),
                ]);
                Ok((key, map))
            },
        )?;

        result.insert("stove".to_string(), stove_state.1);

        Ok(result)
    })
    .map_err(|e| {
        let err_msg = format!("Failed to get current state: {}", e);
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
            r#"
            INSERT INTO temperatures (key, temperature, expected_temperature)
            VALUES (?1, ?2, ?3)
            "#,
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

pub fn get_latest_temperature(
    conn: &Arc<Mutex<Connection>>,
    key: &str,
) -> Result<HashMap<&'static str, String>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.query_row(
            r#"
            SELECT
              temperatures.key,
              COALESCE(labels.label, temperatures.key),
              temperatures.timestamp,
              temperatures.temperature,
              temperatures.expected_temperature
             FROM temperatures
             LEFT JOIN labels ON labels.key = temperatures.key
             WHERE temperatures.key = ?
             ORDER BY temperatures.timestamp DESC LIMIT 1
             "#,
            params![key],
            |row| {
                let mut result = HashMap::from([
                    ("key", row.get::<_, String>(0)?),
                    ("label", row.get::<_, String>(1)?),
                    (
                        "timestamp",
                        NaiveDateTime::parse_from_str(
                            &row.get::<_, String>(2)?,
                            "%Y-%m-%d %H:%M:%S",
                        )
                        .map(|dt| dt.and_utc().to_string())
                        .unwrap(),
                    ),
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

pub fn get_temperatures_since(
    conn: &Arc<Mutex<Connection>>,
    key: &str,
    since: DateTime<Utc>,
) -> Result<Vec<f32>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        let mut stmt = conn.prepare(
            r#"
            SELECT temperature, timestamp
            FROM temperatures
            WHERE key = ?1 AND timestamp >= ?2
            ORDER BY timestamp DESC
            "#,
        )?;

        let timestamp = since.format("%Y-%m-%d %H:%M:%S").to_string();

        let temperatures = stmt
            .query_map(params![key, timestamp], |row| {
                let temperature: f32 = row.get(0)?;
                let timestamp =
                    NaiveDateTime::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d %H:%M:%S")
                        .map(|dt| dt.and_utc().to_string())
                        .unwrap();
                log::debug!(
                    "Collected temperature: {:.1}°C at {} for key {}",
                    temperature,
                    timestamp,
                    key
                );
                Ok(temperature)
            })?
            .collect::<Result<Vec<f32>, _>>()?;

        Ok(temperatures)
    })
    .map_err(|e| {
        let err_msg = format!(
            "Failed to get temperatures since {} for key {}: {}",
            since, key, e
        );
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}

pub fn store_state(
    conn: &Arc<Mutex<Connection>>,
    key: &str,
    state: bool,
) -> Result<(), NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        conn.execute(
            "INSERT INTO states (key, state) VALUES (?1, ?2)",
            params![key, state as i32],
        )
        .map(|_| ())
    })
    .map_err(|e| {
        let err_msg = format!("Failed to store state for key {}: {}", key, e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}

pub fn get_valve_states_and_timestamps(
    conn: &Arc<Mutex<Connection>>,
) -> Result<HashMap<String, (bool, DateTime<Utc>)>, NeuroheatError> {
    db::with_locked_connection(conn, |conn| {
        let mut stmt = conn.prepare(
            r#"
            SELECT key, state, MAX(timestamp) AS latest_timestamp
            FROM states
            GROUP BY key
            "#,
        )?;

        let mut rows = stmt.query([])?;
        let mut result = HashMap::new();

        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let state: i32 = row.get(1)?;
            let timestamp: String = row.get(2)?;

            let datetime = match NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S") {
                Ok(dt) => dt.and_utc(),
                Err(e) => {
                    log::error!(
                        "Failed to parse timestamp for key {}: {}: {}",
                        key,
                        timestamp,
                        e
                    );
                    continue;
                }
            };

            result.insert(key, (state != 0, datetime));
        }

        Ok(result)
    })
    .map_err(|e| {
        let err_msg = format!("Failed to get valve states and timestamps: {}", e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}
