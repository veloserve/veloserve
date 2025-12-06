//! VeloServe - High-performance web server
//!
//! Entry point for the VeloServe server binary.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use veloserve::cli::{self, CacheCommand, ConfigCommand};
use veloserve::config::Config;
use veloserve::server::Server;

/// VeloServe - High-performance web server with integrated PHP support
#[derive(Parser)]
#[command(name = "veloserve")]
#[command(author = "VeloServe Team")]
#[command(version = veloserve::VERSION)]
#[command(about = "High-performance web server with integrated PHP support", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/veloserve/veloserve.toml")]
    config: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the server
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the server
    Stop,
    /// Restart the server
    Restart,
    /// Show server status
    Status,
    /// Cache management commands
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    /// Configuration commands
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            format!("veloserve={},tower_http=debug", log_level).into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Handle commands
    match cli.command {
        Some(Commands::Start { foreground }) => {
            start_server(&cli.config, foreground).await?;
        }
        Some(Commands::Stop) => {
            cli::stop_server()?;
        }
        Some(Commands::Restart) => {
            cli::stop_server()?;
            start_server(&cli.config, false).await?;
        }
        Some(Commands::Status) => {
            cli::show_status()?;
        }
        Some(Commands::Cache { command }) => {
            cli::handle_cache_command(command)?;
        }
        Some(Commands::Config { command }) => {
            cli::handle_config_command(&cli.config, command)?;
        }
        None => {
            // Default: start server in foreground
            start_server(&cli.config, true).await?;
        }
    }

    Ok(())
}

async fn start_server(config_path: &PathBuf, foreground: bool) -> anyhow::Result<()> {
    info!("VeloServe v{} starting...", veloserve::VERSION);

    // Load configuration
    let config = if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        Config::load(config_path)?
    } else {
        info!("Using default configuration");
        Config::default()
    };

    info!(
        "Server configured to listen on {} (HTTP) and {} (HTTPS)",
        config.server.listen,
        config.server.listen_ssl.as_deref().unwrap_or("disabled")
    );

    if !foreground {
        info!("Daemonizing...");
        // In production, we'd fork here
        // For now, just continue running
    }

    // Create and run server
    let server = Server::new(config);

    info!("Starting HTTP server...");
    server.run().await?;

    Ok(())
}

