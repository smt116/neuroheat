use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db;
use crate::heating_configuration::HeatingConfiguration;

pub fn init(conn: &Arc<Mutex<Connection>>, config: &HeatingConfiguration) -> SqliteResult<()> {
    db::with_locked_connection(conn, |conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS rooms (
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
              timestamp TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
              FOREIGN KEY(room_key) REFERENCES rooms(key)
          )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS temperatures_room_key_timestamp_idx ON temperatures (room_key, timestamp)",
            [],
        )?;

        for room in &config.rooms {
            conn.execute(
                "INSERT OR IGNORE INTO rooms (key, label) VALUES (?1, ?2)",
                params![room.key, room.name],
            )?;
        }

        Ok(())
    }).map_err(|e| {
        eprintln!("Failed to initialize database: {}", e);
        std::process::abort();
    })
}

pub fn get_latest_temperature(
    conn: &Arc<Mutex<Connection>>,
    room_key: &str,
) -> SqliteResult<HashMap<&'static str, String>> {
    db::with_locked_connection(conn, |conn| {
        conn.query_row(
            "SELECT
               temperatures.room_key AS key,
               COALESCE(rooms.label, temperatures.room_key) AS label,
               temperatures.timestamp AS timestamp,
               temperatures.temperature AS temperature
             FROM temperatures
             LEFT JOIN rooms ON rooms.key = temperatures.room_key
             WHERE temperatures.room_key = ?
             ORDER BY temperatures.timestamp DESC LIMIT 1",
            params![room_key],
            |row| {
                Ok(HashMap::from([
                    ("key", row.get::<_, String>(0)?),
                    ("label", row.get::<_, String>(1)?),
                    ("timestamp", row.get::<_, String>(2)?),
                    ("temperature", row.get::<_, f32>(3)?.to_string()),
                ]))
            },
        )
    })
    .map_err(|e| match e.downcast_ref::<rusqlite::Error>() {
        Some(rusqlite::Error::QueryReturnedNoRows) => rusqlite::Error::QueryReturnedNoRows,
        _ => rusqlite::Error::ExecuteReturnedResults,
    })
}

pub fn get_latest_temperatures(
    conn: &Arc<Mutex<Connection>>,
) -> SqliteResult<HashMap<String, HashMap<&'static str, String>>> {
    db::with_locked_connection(conn, |conn| {
        let mut stmt = conn.prepare(
            "SELECT
               temperatures.room_key AS key,
               COALESCE(rooms.label, temperatures.room_key) AS label,
               temperatures.timestamp AS timestamp,
               temperatures.temperature AS temperature
             FROM rooms
             LEFT JOIN temperatures ON rooms.key = temperatures.room_key
             AND temperatures.timestamp = (
                 SELECT MAX(timestamp)
                 FROM temperatures
                 WHERE room_key = rooms.key
             )",
        )?;

        let result = stmt
            .query_map([], |row| {
                let key = row.get::<_, String>(0)?;
                let label = row.get::<_, String>(1)?;
                let timestamp = row.get::<_, String>(2)?;
                let temperature = row.get::<_, f32>(3)?.to_string();
                Ok((
                    key,
                    HashMap::from([
                        ("label", label),
                        ("timestamp", timestamp),
                        ("temperature", temperature),
                    ]),
                ))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(result)
    })
    .map_err(|e| match e.downcast_ref::<rusqlite::Error>() {
        Some(rusqlite::Error::QueryReturnedNoRows) => rusqlite::Error::QueryReturnedNoRows,
        _ => rusqlite::Error::ExecuteReturnedResults,
    })
}

pub fn store_temperature(
    conn: &Arc<Mutex<Connection>>,
    room_key: &str,
    temperature: f32,
) -> SqliteResult<()> {
    db::with_locked_connection(conn, |conn| {
        conn.execute(
            "INSERT INTO temperatures (room_key, temperature) VALUES (?1, ?2)",
            params![room_key, temperature],
        )?;

        Ok(())
    })
    .map_err(|_e| rusqlite::Error::ExecuteReturnedResults)
}
