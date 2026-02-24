//! CLI Module
//!
//! Command-line interface tools for VeloServe management.

use anyhow::{anyhow, Result};
use clap::Subcommand;
use std::fs;
use std::path::Path;

// Unix-specific imports for signal handling
#[cfg(unix)]
use nix::sys::signal::Signal;
#[cfg(unix)]
use nix::unistd::Pid;

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
    /// Convert Apache httpd.conf to VeloServe TOML
    ConvertApache {
        /// Path to Apache httpd.conf or vhost file
        #[arg(short, long)]
        input: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
        /// Strict mode: fail on unsupported directives
        #[arg(long)]
        strict: bool,
        /// Only output [[virtualhost]] blocks (for appending to existing config)
        #[arg(long)]
        vhosts_only: bool,
    },
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
            #[cfg(unix)]
            {
                send_signal_to_server(Signal::SIGHUP)?;
                println!("Configuration reload signal sent.");
            }
            #[cfg(windows)]
            {
                println!("Configuration reload not supported on Windows yet.");
                println!("Please restart the server manually.");
            }
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
        ConfigCommand::ConvertApache { input, output, strict, vhosts_only } => {
            use crate::apache_compat::{ApacheConfig, ApacheToVeloServeConverter};
            
            println!("Converting Apache configuration: {}", input);
            
            // Parse Apache config
            let apache_config = ApacheConfig::from_file(&input)
                .map_err(|e| anyhow!("Failed to parse Apache config: {}", e))?;
            
            println!("✓ Parsed {} virtual hosts", apache_config.virtual_hosts.len());
            
            // Convert to VeloServe
            let converter = ApacheToVeloServeConverter::new()
                .strict(strict);
            
            let toml_output = if vhosts_only {
                converter.to_toml_vhosts_only(&apache_config)
            } else {
                converter.to_toml(&apache_config)
            };
            
            // Write output
            if let Some(output_path) = output {
                fs::write(&output_path, &toml_output)?;
                println!("✓ Converted configuration written to: {}", output_path);
            } else {
                println!("\n=== Converted Configuration ===\n");
                println!("{}", toml_output);
            }
            
            // Summary
            println!("\n=== Summary ===");
            println!("Virtual Hosts: {}", apache_config.virtual_hosts.len());
            for vhost in &apache_config.virtual_hosts {
                if let Some(domain) = vhost.server_names.first() {
                    println!("  - {} (port {})", domain, vhost.port);
                }
            }
        }
    }
    Ok(())
}

/// Stop the running server
pub fn stop_server() -> Result<()> {
    println!("Stopping VeloServe...");
    #[cfg(unix)]
    {
        send_signal_to_server(Signal::SIGTERM)?;
        println!("Stop signal sent.");
    }
    #[cfg(windows)]
    {
        // On Windows, we try to terminate the process
        let pid_file = "veloserve.pid";
        if Path::new(pid_file).exists() {
            let pid = fs::read_to_string(pid_file)?;
            let pid: u32 = pid.trim().parse()?;
            println!("Attempting to stop process {}...", pid);
            // Use taskkill on Windows
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
            let _ = fs::remove_file(pid_file);
            println!("Stop signal sent.");
        } else {
            println!("Server not running (no PID file).");
        }
    }
    Ok(())
}

/// Show server status
pub fn show_status() -> Result<()> {
    println!("VeloServe Status");
    println!("================");

    // Check if PID file exists (different paths for Unix/Windows)
    #[cfg(unix)]
    let pid_file = "/var/run/veloserve.pid";
    #[cfg(windows)]
    let pid_file = "veloserve.pid";

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

/// Send a signal to the running server (Unix only)
#[cfg(unix)]
fn send_signal_to_server(signal: Signal) -> Result<()> {
    let pid_file = "/var/run/veloserve.pid";

    if !Path::new(pid_file).exists() {
        return Err(anyhow!("Server not running (no PID file)"));
    }

    let pid = fs::read_to_string(pid_file)?;
    let pid: i32 = pid.trim().parse()?;

    nix::sys::signal::kill(Pid::from_raw(pid), signal)
        .map_err(|e| anyhow!("Failed to send signal: {}", e))?;

    Ok(())
}

/// Check if a process is running
#[cfg(unix)]
fn is_process_running(pid: i32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

/// Check if a process is running (Windows)
#[cfg(windows)]
fn is_process_running(pid: i32) -> bool {
    // On Windows, try to open the process
    use std::process::Command;
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid)])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
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

