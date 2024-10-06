use crate::heating_configuration::HeatingConfiguration;
use crate::relay::read_relay_states;
use crate::temperature_sensor::read_temperatures;

use rusqlite::Connection;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};

async fn create_temperature_job(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Job, Box<dyn Error + Send + Sync>> {
    let config_clone = Arc::clone(&config);
    let conn_clone = Arc::clone(&conn);

    log::info!("Creating job for reading temperatures: 0 */5 * * * *");

    let job = Job::new_async("0 */5 * * * *", move |_uuid, _l| {
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

async fn create_relay_job(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Job, Box<dyn Error + Send + Sync>> {
    let config_clone = Arc::clone(&config);
    let conn_clone = Arc::clone(&conn);

    log::info!("Creating job for reading relays: 0 */15 * * * *");

    let job = Job::new_async("0 */15 * * * *", move |_uuid, _l| {
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

pub async fn start_scheduler(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<JobScheduler, Box<dyn Error + Send + Sync>> {
    let scheduler = JobScheduler::new().await?;

    let temperature_job = create_temperature_job(Arc::clone(&config), Arc::clone(&conn)).await?;
    let relay_job = create_relay_job(Arc::clone(&config), Arc::clone(&conn)).await?;

    scheduler.add(temperature_job).await?;
    scheduler.add(relay_job).await?;
    scheduler.start().await?;

    Ok(scheduler)
}
