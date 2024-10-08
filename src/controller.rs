use crate::error::NeuroheatError;
use crate::heating_configuration::HeatingConfiguration;
use crate::repo;
use chrono::{Duration, Utc};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// The minimum area in square meters that a stove requires
/// when turning on heating. If valves are open for a smaller
/// area, the stove would turn on and off frequently which is
/// not desired (e.g., higher gas consumption).
const STOVE_ACTIVATION_AREA: f32 = 16.0;

/// The duration in minutes to wait after a valve state change
/// before considering it for stove activation. This ensures
/// the valve has had enough time to open for heating.
const STOVE_ACTIVATION_DELAY_MINUTES: i64 = 2;

/// The duration in minutes to look back for temperature readings
/// when calculating average value for valve control.
const TEMPERATURE_LOOKBACK_MINUTES: i64 = 10;

/// The minimum number of temperature readings required for valve
/// control.
const MIN_TEMPERATURE_READINGS: usize = 3;

pub async fn update_valves(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<(), NeuroheatError> {
    let now = Utc::now();
    let ten_minutes_ago = now - Duration::minutes(TEMPERATURE_LOOKBACK_MINUTES);

    for room in &config.rooms {
        let temperatures = match repo::get_temperatures_since(&conn, &room.key, ten_minutes_ago) {
            Ok(temps) => temps,
            Err(e) => {
                log::error!("Failed to get temperatures for room {}: {}", room.name, e);
                continue;
            }
        };

        if temperatures.len() < MIN_TEMPERATURE_READINGS {
            log::error!("Not enough temperature readings for room {}", room.name);
            continue;
        }

        let average_temperature: f32 = temperatures.iter().sum::<f32>() / temperatures.len() as f32;

        let expected_temperature = match room.get_expected_temperature() {
            Some(temp) => temp,
            None => {
                log::error!("No expected temperature found for room {}", room.name);
                continue;
            }
        };

        if let Some(valve_controller) = &room.valve_reader {
            let desired_state = average_temperature < expected_temperature;
            let current_state = match valve_controller.read_state() {
                Ok(state) => state,
                Err(e) => {
                    log::error!("Failed to read valve state for room {}: {}", room.name, e);
                    continue;
                }
            };

            if current_state != desired_state {
                log::info!(
                    "Room: {}, Average Temperature: {:.1}°C is {} Expected Temperature: {:.1}°C. Turning valve {}.",
                    room.name,
                    average_temperature,
                    if desired_state { "less than" } else { "greater than or equal to" },
                    expected_temperature,
                    if desired_state { "ON" } else { "OFF" }
                );
                if let Err(e) = valve_controller.set_state(desired_state) {
                    log::error!("Failed to set valve state for room {}: {}", room.name, e);
                    continue;
                }
                if let Err(e) = repo::store_state(&conn, &room.key, desired_state) {
                    log::error!("Failed to store state for room {}: {}", room.name, e);
                    continue;
                }
            } else {
                log::debug!(
                    "Room: {}, Average Temperature: {:.1}°C is {} Expected Temperature: {:.1}°C. Valve is already {}.",
                    room.name,
                    average_temperature,
                    if desired_state { "less than" } else { "greater than or equal to" },
                    expected_temperature,
                    if desired_state { "ON" } else { "OFF" }
                );
            }
        } else {
            log::error!("No valve controller found for room {}", room.name);
            continue;
        }
    }

    Ok(())
}

pub async fn update_stove_state(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<(), NeuroheatError> {
    let now = Utc::now();
    let valve_states = repo::get_valve_states_and_timestamps(&conn)?;

    let mut total_open_area = 0.0;

    for room in &config.rooms {
        if let Some((valve_state, last_change)) = valve_states.get(&room.key) {
            log::debug!(
                "Room: {}, Valve State: {}, Last Change: {}",
                room.name,
                valve_state,
                last_change
            );

            if *valve_state
                && now.signed_duration_since(*last_change)
                    >= Duration::minutes(STOVE_ACTIVATION_DELAY_MINUTES)
            {
                total_open_area += room.area;
                log::debug!(
                    "Room: {}, Area: {:.1} m² added to total open area. New total: {:.1} m²",
                    room.name,
                    room.area,
                    total_open_area
                );
            }
        }
    }

    // Control the stove based on the total open area
    if let Some(stove_controller) = &config.stove_reader {
        let stove_state = stove_controller.read_state()?;
        let desired_stove_state = total_open_area >= STOVE_ACTIVATION_AREA;

        if stove_state != desired_stove_state {
            if desired_stove_state {
                log::info!(
                    "Total open area is {:.1} m². Turning stove ON.",
                    total_open_area
                );
            } else {
                log::info!(
                    "Total open area is {:.1} m². Turning stove OFF.",
                    total_open_area
                );
            }

            stove_controller.set_state(desired_stove_state)?;
            repo::store_state(&conn, "stove", desired_stove_state)?;
        } else {
            log::debug!(
                "Total open area is {:.1} m². Stove is already {}.",
                total_open_area,
                if desired_stove_state { "ON" } else { "OFF" }
            );
        }
    } else {
        return Err(NeuroheatError::ConfigurationError(
            "No stove controller found".to_string(),
        ));
    }

    Ok(())
}
