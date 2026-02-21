//! veloserve-php - Standalone PHP Worker Binary
//!
//! Similar to lsphp (LiteSpeed PHP), this binary runs as a standalone
//! PHP worker that communicates with VeloServe via sockets.
//!
//! Usage:
//!   veloserve-php --socket /run/veloserve/php.sock
//!   veloserve-php --socket 127.0.0.1:9000
//!   veloserve-php --user cpaneluser --socket /run/veloserve/user.sock

use std::env;
use std::path::PathBuf;
use std::process::exit;

mod server;
mod worker;
mod pool;
mod protocol;

use server::PhpWorkerServer;

/// Default socket path for veloserve-php
pub const DEFAULT_SOCKET: &str = "/run/veloserve/php.sock";

/// Default number of PHP workers
pub const DEFAULT_WORKERS: usize = 8;

/// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage() {
    eprintln!("veloserve-php {} - High-performance PHP worker for VeloServe", VERSION);
    eprintln!();
    eprintln!("Usage: veloserve-php [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -s, --socket <PATH|ADDR>  Socket path (Unix) or address:port (TCP)");
    eprintln!("                            [default: {}]", DEFAULT_SOCKET);
    eprintln!("  -u, --user <USER>         Run as specific user (cPanel username)");
    eprintln!("  -w, --workers <N>         Number of PHP workers [default: {}]", DEFAULT_WORKERS);
    eprintln!("  -m, --memory <LIMIT>      PHP memory limit [default: 256M]");
    eprintln!("  -t, --timeout <SECS>      Max execution time [default: 30]");
    eprintln!("  -c, --config <FILE>       PHP ini file path");
    eprintln!("  -d, --daemon              Run as daemon");
    eprintln!("  -p, --pid <FILE>          PID file path");
    eprintln!("  -v, --verbose             Verbose logging");
    eprintln!("  -h, --help                Show this help");
    eprintln!("  -V, --version             Show version");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  veloserve-php");
    eprintln!("  veloserve-php -s /run/veloserve/php.sock -w 16");
    eprintln!("  veloserve-php -u cpaneluser -s /run/veloserve/user.sock");
    eprintln!("  veloserve-php -s 127.0.0.1:9000 -m 512M");
}

fn print_version() {
    println!("veloserve-php {}", VERSION);
    println!("Copyright (C) 2026 VeloServe Project");
    println!("License: MIT");
}

pub struct Config {
    pub socket: String,
    pub user: Option<String>,
    pub workers: usize,
    pub memory_limit: String,
    pub max_execution_time: u32,
    pub php_ini: Option<PathBuf>,
    pub daemon: bool,
    pub pid_file: Option<PathBuf>,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket: DEFAULT_SOCKET.to_string(),
            user: None,
            workers: DEFAULT_WORKERS,
            memory_limit: "256M".to_string(),
            max_execution_time: 30,
            php_ini: None,
            daemon: false,
            pid_file: None,
            verbose: false,
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config::default();
    let args: Vec<String> = env::args().skip(1).collect();
    
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-s" | "--socket" => {
                i += 1;
                if i < args.len() {
                    config.socket = args[i].clone();
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < args.len() {
                    config.user = Some(args[i].clone());
                }
            }
            "-w" | "--workers" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse() {
                        config.workers = n;
                    }
                }
            }
            "-m" | "--memory" => {
                i += 1;
                if i < args.len() {
                    config.memory_limit = args[i].clone();
                }
            }
            "-t" | "--timeout" => {
                i += 1;
                if i < args.len() {
                    if let Ok(n) = args[i].parse() {
                        config.max_execution_time = n;
                    }
                }
            }
            "-c" | "--config" => {
                i += 1;
                if i < args.len() {
                    config.php_ini = Some(PathBuf::from(&args[i]));
                }
            }
            "-d" | "--daemon" => {
                config.daemon = true;
            }
            "-p" | "--pid" => {
                i += 1;
                if i < args.len() {
                    config.pid_file = Some(PathBuf::from(&args[i]));
                }
            }
            "-v" | "--verbose" => {
                config.verbose = true;
            }
            "-h" | "--help" => {
                print_usage();
                exit(0);
            }
            "-V" | "--version" => {
                print_version();
                exit(0);
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                exit(1);
            }
        }
        i += 1;
    }
    
    config
}

fn main() {
    let config = parse_args();
    
    // Initialize logging
    if config.verbose {
        println!("[veloserve-php] Starting with config:");
        println!("  Socket: {}", config.socket);
        println!("  Workers: {}", config.workers);
        println!("  Memory: {}", config.memory_limit);
        println!("  Timeout: {}s", config.max_execution_time);
        if let Some(ref user) = config.user {
            println!("  User: {}", user);
        }
    }
    
    // Create and run the PHP worker server
    let server = PhpWorkerServer::new(config);
    
    if let Err(e) = server.run() {
        eprintln!("[veloserve-php] Error: {}", e);
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.socket, DEFAULT_SOCKET);
        assert_eq!(config.workers, DEFAULT_WORKERS);
        assert_eq!(config.memory_limit, "256M");
        assert_eq!(config.max_execution_time, 30);
    }
}
