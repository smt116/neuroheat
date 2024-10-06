use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use warp::Filter;

use crate::repo;

const LOGGER_TARGET: &str = concat!(env!("CARGO_PKG_NAME"), "::api");

pub async fn start_server(conn: Arc<Mutex<Connection>>, port: u16) {
    let log = warp::log(LOGGER_TARGET);

    let temperatures = warp::path!("api" / "temperatures")
        .and(warp::get())
        .and(with_db(conn.clone()))
        .and_then(get_all_temperatures)
        .with(log);

    let temperature_by_room = warp::path!("api" / "temperatures" / String)
        .and(warp::get())
        .and(with_db(conn.clone()))
        .and_then(get_temperature_by_room)
        .with(log);

    let routes = temperature_by_room.or(temperatures);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

async fn get_all_temperatures(
    conn: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    match repo::get_latest_temperatures(&conn) {
        Ok(result) => Ok(warp::reply::json(&result)),
        Err(e) => {
            log::error!("Failed to get all temperatures: {}", e);
            Err(warp::reject::not_found())
        }
    }
}

async fn get_temperature_by_room(
    room: String,
    conn: Arc<Mutex<Connection>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    match repo::get_latest_temperature(&conn, &room) {
        Ok(result) => Ok(warp::reply::json(&result)),
        Err(e) => {
            log::error!("Failed to get temperature for room {}: {}", room, e);
            Err(warp::reject::not_found())
        }
    }
}

fn with_db(
    conn: Arc<Mutex<Connection>>,
) -> impl Filter<Extract = (Arc<Mutex<Connection>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || conn.clone())
}
