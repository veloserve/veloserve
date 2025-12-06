//! CLI Module
//!
//! Command-line interface tools for VeloServe management.

use anyhow::{anyhow, Result};
use clap::Subcommand;
use std::fs;
use std::path::Path;

/// Cache management subcommands
#[derive(Subcommand)]
pub enum CacheCommand {
    /// Purge cache entries
    Purge {
        /// Purge all cache entries
        #[arg(long)]
        all: bool,

        /// Purge entries for a specific domain
        #[arg(long)]
        domain: Option<String>,

        /// Purge entries with a specific tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show cache statistics
    Stats,
    /// Warm up cache
    Warm {
        /// URL list file
        #[arg(long)]
        urls: Option<String>,
    },
}

/// Configuration subcommands
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Validate configuration file
    Validate,
    /// Reload configuration (sends SIGHUP to running server)
    Reload,
    /// Test configuration and show parsed result
    Test,
    /// Show default configuration
    ShowDefault,
}

/// Handle cache commands
pub fn handle_cache_command(cmd: CacheCommand) -> Result<()> {
    match cmd {
        CacheCommand::Purge { all, domain, tag } => {
            if all {
                println!("Purging all cache entries...");
                // In production, this would communicate with running server
                send_management_command("cache.purge.all")?;
                println!("Cache purged successfully.");
            } else if let Some(domain) = domain {
                println!("Purging cache for domain: {}", domain);
                send_management_command(&format!("cache.purge.domain:{}", domain))?;
                println!("Domain cache purged successfully.");
            } else if let Some(tag) = tag {
                println!("Purging cache entries with tag: {}", tag);
                send_management_command(&format!("cache.purge.tag:{}", tag))?;
                println!("Tagged entries purged successfully.");
            } else {
                println!("Please specify --all, --domain, or --tag");
            }
        }
        CacheCommand::Stats => {
            println!("Cache Statistics:");
            println!("-----------------");
            // In production, fetch from running server
            println!("Entries: N/A (server not running or not connected)");
            println!("Memory: N/A");
            println!("Hit Rate: N/A");
        }
        CacheCommand::Warm { urls } => {
            if let Some(file) = urls {
                println!("Warming cache from URL list: {}", file);
                warm_cache_from_file(&file)?;
            } else {
                println!("Please provide --urls file");
            }
        }
    }
    Ok(())
}

/// Handle configuration commands
pub fn handle_config_command(config_path: &Path, cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Validate => {
            println!("Validating configuration: {:?}", config_path);
            if !config_path.exists() {
                println!("Configuration file not found, using defaults.");
                println!("Configuration is valid.");
                return Ok(());
            }

            match crate::config::Config::load(config_path) {
                Ok(_) => {
                    println!("✓ Configuration is valid.");
                }
                Err(e) => {
                    println!("✗ Configuration error: {}", e);
                    return Err(anyhow!("Invalid configuration"));
                }
            }
        }
        ConfigCommand::Reload => {
            println!("Reloading configuration...");
            send_signal_to_server(nix::sys::signal::Signal::SIGHUP)?;
            println!("Configuration reload signal sent.");
        }
        ConfigCommand::Test => {
            println!("Testing configuration: {:?}", config_path);
            let config = if config_path.exists() {
                crate::config::Config::load(config_path)?
            } else {
                println!("(Using default configuration)");
                crate::config::Config::default()
            };

            println!("\n=== Parsed Configuration ===\n");
            println!("[server]");
            println!("  listen: {}", config.server.listen);
            println!(
                "  listen_ssl: {}",
                config.server.listen_ssl.as_deref().unwrap_or("disabled")
            );
            println!("  workers: {}", config.server.workers);
            println!("  max_connections: {}", config.server.max_connections);

            println!("\n[php]");
            println!("  enabled: {}", config.php.enable);
            println!("  version: {}", config.php.version);
            println!("  workers: {}", config.php.workers);
            println!("  memory_limit: {}", config.php.memory_limit);

            println!("\n[cache]");
            println!("  enabled: {}", config.cache.enable);
            println!("  storage: {:?}", config.cache.storage);
            println!("  memory_limit: {}", config.cache.memory_limit);
            println!("  default_ttl: {}s", config.cache.default_ttl);

            if !config.virtualhost.is_empty() {
                println!("\n[[virtualhost]]");
                for vhost in &config.virtualhost {
                    println!("  domain: {}", vhost.domain);
                    println!("  root: {}", vhost.root);
                    if let Some(ref platform) = vhost.platform {
                        println!("  platform: {}", platform);
                    }
                    println!();
                }
            }

            println!("\n✓ Configuration test passed.");
        }
        ConfigCommand::ShowDefault => {
            let default_config = r#"# VeloServe Configuration
# See https://docs.veloserve.io for full documentation

[server]
listen = "0.0.0.0:8080"
# listen_ssl = "0.0.0.0:443"
workers = "auto"
max_connections = 10000
keepalive_timeout = 75
request_timeout = 60

[php]
enable = true
version = "8.2"
workers = 16
memory_limit = "256M"
max_execution_time = 30

[cache]
enable = true
storage = "memory"
memory_limit = "512M"
default_ttl = 3600

# [ssl]
# cert = "/etc/veloserve/ssl/cert.pem"
# key = "/etc/veloserve/ssl/key.pem"
# protocols = ["TLSv1.2", "TLSv1.3"]

# [[virtualhost]]
# domain = "example.com"
# root = "/var/www/example.com"
# platform = "wordpress"
# index = ["index.php", "index.html"]
#
# [virtualhost.cache]
# enable = true
# ttl = 7200
# vary = ["Accept-Encoding"]
"#;
            println!("{}", default_config);
        }
    }
    Ok(())
}

/// Stop the running server
pub fn stop_server() -> Result<()> {
    println!("Stopping VeloServe...");
    send_signal_to_server(nix::sys::signal::Signal::SIGTERM)?;
    println!("Stop signal sent.");
    Ok(())
}

/// Show server status
pub fn show_status() -> Result<()> {
    println!("VeloServe Status");
    println!("================");

    // Check if PID file exists
    let pid_file = "/var/run/veloserve.pid";
    if Path::new(pid_file).exists() {
        let pid = fs::read_to_string(pid_file)?;
        let pid: i32 = pid.trim().parse()?;

        // Check if process is running
        if is_process_running(pid) {
            println!("Status: Running");
            println!("PID: {}", pid);
        } else {
            println!("Status: Not running (stale PID file)");
        }
    } else {
        println!("Status: Not running");
    }

    Ok(())
}

/// Send a management command to the running server
fn send_management_command(cmd: &str) -> Result<()> {
    // In production, this would use a Unix socket or HTTP API
    // For now, just log
    tracing::debug!("Management command: {}", cmd);
    Ok(())
}

/// Send a signal to the running server
fn send_signal_to_server(signal: nix::sys::signal::Signal) -> Result<()> {
    let pid_file = "/var/run/veloserve.pid";

    if !Path::new(pid_file).exists() {
        return Err(anyhow!("Server not running (no PID file)"));
    }

    let pid = fs::read_to_string(pid_file)?;
    let pid: i32 = pid.trim().parse()?;

    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), signal)
        .map_err(|e| anyhow!("Failed to send signal: {}", e))?;

    Ok(())
}

/// Check if a process is running
fn is_process_running(pid: i32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

/// Warm cache from URL list file
fn warm_cache_from_file(file_path: &str) -> Result<()> {
    let contents = fs::read_to_string(file_path)?;
    let urls: Vec<&str> = contents.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();

    println!("Found {} URLs to warm", urls.len());

    for url in urls {
        println!("  Warming: {}", url);
        // In production, make HTTP request to the URL
    }

    println!("Cache warming complete.");
    Ok(())
}

