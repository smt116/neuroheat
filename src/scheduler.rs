use crate::controller;
use crate::heating_configuration::HeatingConfiguration;
use crate::relay::read_relay_states;
use crate::temperature_sensor::read_temperatures;

use rusqlite::Connection;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};

/// How often to update the relay states for valves based on. e.g.,
/// the room temperatures.
const VALVE_CONTROLLER_CRON: &str = "30 */2 * * * *";

/// How often to update the stove state based on the open valve areas.
const STOVE_CONTROLLER_CRON: &str = "0 */5 * * * *";

/// How often to read the relay states. They are also stored
/// when updating their state.
const RELAY_CRON: &str = "45 */15 * * * *";

/// How often to read temperatures from the sensors.
const TEMPERATURE_CRON: &str = "0 */2 * * * *";

async fn temperature_job(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Job, Box<dyn Error + Send + Sync>> {
    let config_clone = Arc::clone(&config);
    let conn_clone = Arc::clone(&conn);

    log::info!(
        "Creating job for reading temperatures: {}",
        TEMPERATURE_CRON
    );

    let job = Job::new_async(TEMPERATURE_CRON, move |_uuid, _l| {
        let config_clone = Arc::clone(&config_clone);
        let conn_clone = Arc::clone(&conn_clone);

        Box::pin(async move {
            if let Err(e) = read_temperatures(config_clone.clone(), conn_clone.clone()).await {
                log::error!("Error in temperature reading task: {}", e);
            }
        })
    })?;

    Ok(job)
}

async fn relay_job(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Job, Box<dyn Error + Send + Sync>> {
    let config_clone = Arc::clone(&config);
    let conn_clone = Arc::clone(&conn);

    log::info!("Creating job for reading relays: {}", RELAY_CRON);

    let job = Job::new_async(RELAY_CRON, move |_uuid, _l| {
        let config_clone = Arc::clone(&config_clone);
        let conn_clone = Arc::clone(&conn_clone);

        Box::pin(async move {
            if let Err(e) = read_relay_states(config_clone, conn_clone).await {
                log::error!("Error in relay state reading task: {}", e);
            }
        })
    })?;

    Ok(job)
}

async fn valve_controller_job(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Job, Box<dyn Error + Send + Sync>> {
    let config_clone = Arc::clone(&config);
    let conn_clone = Arc::clone(&conn);

    log::info!(
        "Creating job for controlling heating: {}",
        VALVE_CONTROLLER_CRON
    );

    let job = Job::new_async(VALVE_CONTROLLER_CRON, move |_uuid, _l| {
        let config_clone = Arc::clone(&config_clone);
        let conn_clone = Arc::clone(&conn_clone);

        Box::pin(async move {
            if let Err(e) = controller::update_valves(config_clone, conn_clone).await {
                log::error!("Error in heating control task: {}", e);
            }
        })
    })?;

    Ok(job)
}

async fn stove_controller_job(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Job, Box<dyn Error + Send + Sync>> {
    let config_clone = Arc::clone(&config);
    let conn_clone = Arc::clone(&conn);

    log::info!(
        "Creating job for controlling stove: {}",
        STOVE_CONTROLLER_CRON
    );

    let job = Job::new_async(STOVE_CONTROLLER_CRON, move |_uuid, _l| {
        let config_clone = Arc::clone(&config_clone);
        let conn_clone = Arc::clone(&conn_clone);

        Box::pin(async move {
            if let Err(e) = controller::update_stove_state(config_clone, conn_clone).await {
                log::error!("Error in stove control task: {}", e);
            }
        })
    })?;

    Ok(job)
}

pub async fn start_scheduler(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<JobScheduler, Box<dyn Error + Send + Sync>> {
    let scheduler = JobScheduler::new().await?;

    let temperature_job = temperature_job(Arc::clone(&config), Arc::clone(&conn)).await?;
    let relay_job = relay_job(Arc::clone(&config), Arc::clone(&conn)).await?;
    let valve_controller_job = valve_controller_job(Arc::clone(&config), Arc::clone(&conn)).await?;
    let stove_controller_job = stove_controller_job(Arc::clone(&config), Arc::clone(&conn)).await?;

    scheduler.add(temperature_job).await?;
    scheduler.add(relay_job).await?;
    scheduler.add(valve_controller_job).await?;
    scheduler.add(stove_controller_job).await?;
    scheduler.start().await?;

    Ok(scheduler)
}
