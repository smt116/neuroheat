use crate::heating_configuration::HeatingConfiguration;
use crate::temperature_sensor::read_temperatures;

use rusqlite::Connection;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn start_scheduler(
    config: Arc<HeatingConfiguration>,
    conn: Arc<Mutex<Connection>>,
) -> Result<JobScheduler, Box<dyn Error + Send + Sync>> {
    let scheduler = JobScheduler::new().await?;

    let job = Job::new_async("0 */5 * * * *", move |_uuid, _l| {
        let config_clone = Arc::clone(&config);
        let conn_clone = Arc::clone(&conn);

        Box::pin(async move {
            if let Err(e) = read_temperatures(config_clone, conn_clone).await {
                log::error!("Error in temperature reading task: {}", e);
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    Ok(scheduler)
}
