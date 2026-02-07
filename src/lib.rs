pub mod cli;
pub mod config;
pub mod deploy;
pub mod services;
pub mod systemd;
pub mod templates;
pub mod util;

use anyhow::Result;
use clap::Parser;
use tracing::info;

pub fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    util::init_logging(cli.verbose);

    let mut app_config = config::load()?;
    let config_path = config::resolve_config_path()?;

    match cli.command {
        cli::Commands::Deploy { opts } => {
            info!("starting deploy");
            deploy::run(opts, &config_path, &mut app_config)?;
        }
        cli::Commands::Start => {
            info!("starting services");
            services::start(&config_path, &app_config)?;
        }
        cli::Commands::Stop => {
            info!("stopping services");
            services::stop(&config_path, &app_config)?;
        }
        cli::Commands::Clean { opts } => {
            info!("cleaning deployment");
            services::clean(opts, &config_path, &app_config)?;
        }
        cli::Commands::Status => {
            info!("checking status");
            services::status(&app_config)?;
        }
    }

    Ok(())
}
