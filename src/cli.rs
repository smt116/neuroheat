use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "neuroheat")]
#[command(author = "Maciej Małecki <maciej@smefju.pl>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Neuroheat system")]
pub struct Args {
    #[arg(long, default_value = "info")]
    pub log_level: String,

    #[arg(long, default_value = "neuroheat.db")]
    pub database_path: String,

    #[arg(long, default_value_t = 3030)]
    pub api_port: u16,

    #[arg(long, default_value = "heating_config.json")]
    pub heating_config_path: String,
}

pub fn parse_log_level(args: &Args) -> log::LevelFilter {
    match args.log_level.to_lowercase().as_str() {
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    }
}
