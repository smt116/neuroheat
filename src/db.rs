use rusqlite::Connection;
use std::error::Error;
use std::sync::{Arc, Mutex};

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
