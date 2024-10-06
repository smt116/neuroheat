mod api;
mod cli;
mod db;
mod error;
mod heating_configuration;
mod relay;
mod repo;
mod scheduler;
mod temperature_sensor;

use heating_configuration::HeatingConfiguration;

use clap::Parser;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = cli::Args::parse();
    let log_level = cli::parse_log_level(&args);

    env_logger::Builder::new().filter(None, log_level).init();

    let config_path = &args.heating_config_path;
    let config = Arc::new(HeatingConfiguration::from_file(config_path)?);
    let conn = db::open(args.database_path);
    let shared_conn = Arc::new(Mutex::new(conn));

    // initialize database if necessary
    db::init(&shared_conn, &config)?;

    // start scheduler (e.g., reading data from sensors)
    scheduler::start_scheduler(Arc::clone(&config), Arc::clone(&shared_conn)).await?;

    // start API server
    api::start_server(Arc::clone(&shared_conn), args.api_port).await;

    Ok(())
}
