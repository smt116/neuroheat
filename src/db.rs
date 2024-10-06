use rusqlite::{params, Connection};
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::error::NeuroheatError;
use crate::heating_configuration::HeatingConfiguration;

pub fn open(path: String) -> Connection {
    match Connection::open(path) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to open connection: {}", e);
            std::process::abort();
        }
    }
}

pub fn with_locked_connection<F, T>(
    conn: &Arc<Mutex<Connection>>,
    f: F,
) -> Result<T, Box<dyn Error + Send + Sync>>
where
    F: FnOnce(&Connection) -> rusqlite::Result<T>,
{
    let conn = conn.lock().map_err(|e| {
        let err_msg = format!("Failed to lock connection: {}", e);
        Box::<dyn Error + Send + Sync>::from(err_msg)
    })?;
    f(&conn).map_err(|e| Box::<dyn Error + Send + Sync>::from(e))
}

pub fn init(
    conn: &Arc<Mutex<Connection>>,
    config: &HeatingConfiguration,
) -> Result<(), NeuroheatError> {
    with_locked_connection(conn, |conn| {
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
              key TEXT NOT NULL,
              temperature REAL NOT NULL,
              expected_temperature REAL,
              timestamp TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
              FOREIGN KEY(key) REFERENCES labels(key)
          )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS temperatures_key_timestamp_idx ON temperatures (key, timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS states (
              id INTEGER PRIMARY KEY,
              key TEXT NOT NULL,
              state INTEGER NOT NULL,
              timestamp TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
              FOREIGN KEY(key) REFERENCES labels(key)
          )",
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

        // Insert a value for the stove as "Stove"
        conn.execute(
            "INSERT OR IGNORE INTO labels (key, label) VALUES (?1, ?2)",
            params!["stove", "Stove"],
        )?;

        Ok(())
    })
    .map_err(|e| {
        let err_msg = format!("Failed to initialize database: {}", e);
        log::error!("{}", err_msg);
        NeuroheatError::DatabaseError(err_msg)
    })
}
