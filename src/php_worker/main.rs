//! vephp - VeloServe PHP Worker
//!
//! Persistent PHP worker process similar to LiteSpeed's lsphp.
//! Communicates with VeloServe over Unix sockets for high-performance
//! PHP execution without per-request process spawning.
//!
//! Auto-discovers cPanel EA-PHP, CloudLinux alt-PHP, or system PHP.
//!
//! Usage:
//!   vephp --socket /run/veloserve/php.sock
//!   vephp --socket 127.0.0.1:9000
//!   vephp --user cpaneluser --socket /run/veloserve/user.sock
//!   vephp --php /opt/cpanel/ea-php83/root/usr/bin/php-cgi

use std::env;
use std::path::PathBuf;
use std::process::exit;

mod server;
mod worker;
mod pool;
mod protocol;

use server::PhpWorkerServer;

pub const DEFAULT_SOCKET: &str = "/run/veloserve/php.sock";
pub const DEFAULT_WORKERS: usize = 8;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage() {
    eprintln!("vephp {} - VeloServe PHP Worker (like lsphp)", VERSION);
    eprintln!();
    eprintln!("Usage: vephp [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -s, --socket <PATH|ADDR>  Socket path (Unix) or address:port (TCP)");
    eprintln!("                            [default: {}]", DEFAULT_SOCKET);
    eprintln!("  -u, --user <USER>         Run as specific user (cPanel username)");
    eprintln!("  -w, --workers <N>         Number of PHP workers [default: {}]", DEFAULT_WORKERS);
    eprintln!("  -m, --memory <LIMIT>      PHP memory limit [default: 256M]");
    eprintln!("  -t, --timeout <SECS>      Max execution time [default: 30]");
    eprintln!("  -c, --config <FILE>       PHP ini file path");
    eprintln!("  --php <PATH>              Path to php-cgi binary (auto-detects EA-PHP)");
    eprintln!("  -d, --daemon              Run as daemon");
    eprintln!("  -p, --pid <FILE>          PID file path");
    eprintln!("  -v, --verbose             Verbose logging");
    eprintln!("  -h, --help                Show this help");
    eprintln!("  -V, --version             Show version");
    eprintln!();
    eprintln!("PHP Discovery (automatic):");
    eprintln!("  1. --php flag if specified");
    eprintln!("  2. cPanel EA-PHP: /opt/cpanel/ea-php83/root/usr/bin/php-cgi");
    eprintln!("  3. CloudLinux:    /opt/alt/php83/usr/bin/php-cgi");
    eprintln!("  4. System:        /usr/bin/php-cgi");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  vephp                                         # Auto-detect PHP, default socket");
    eprintln!("  vephp -s /run/veloserve/php.sock -w 16        # 16 workers");
    eprintln!("  vephp -u cpaneluser -s /run/veloserve/u.sock  # Per-user isolation");
    eprintln!("  vephp --php /opt/cpanel/ea-php83/root/usr/bin/php-cgi");
}

fn print_version() {
    println!("vephp {}", VERSION);
    println!("VeloServe PHP Worker - like lsphp for LiteSpeed");
    println!("License: MIT");
}

pub struct Config {
    pub socket: String,
    pub user: Option<String>,
    pub workers: usize,
    pub memory_limit: String,
    pub max_execution_time: u32,
    pub php_ini: Option<PathBuf>,
    pub php_binary: Option<PathBuf>,
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
            php_binary: None,
            daemon: false,
            pid_file: None,
            verbose: false,
        }
    }
}

impl Config {
    /// Resolve which PHP binary to use.
    /// Priority: explicit --php flag > EA-PHP > CloudLinux alt-PHP > system php-cgi
    pub fn resolve_php_binary(&self) -> PathBuf {
        if let Some(ref explicit) = self.php_binary {
            if explicit.exists() {
                return explicit.clone();
            }
            eprintln!("[vephp] WARNING: Specified PHP binary not found: {:?}", explicit);
        }

        let php_versions = ["84", "83", "82", "81", "80", "74"];

        // cPanel EA-PHP (newest first)
        for ver in &php_versions {
            let path = PathBuf::from(format!("/opt/cpanel/ea-php{}/root/usr/bin/php-cgi", ver));
            if path.exists() {
                return path;
            }
        }

        // CloudLinux alt-PHP
        for ver in &php_versions {
            let path = PathBuf::from(format!("/opt/alt/php{}/usr/bin/php-cgi", ver));
            if path.exists() {
                return path;
            }
        }

        // System paths
        for path_str in &["/usr/bin/php-cgi", "/usr/bin/php", "/usr/local/bin/php-cgi"] {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return path;
            }
        }

        PathBuf::from("php-cgi")
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
            "--php" => {
                i += 1;
                if i < args.len() {
                    config.php_binary = Some(PathBuf::from(&args[i]));
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

    let php_binary = config.resolve_php_binary();

    println!("[vephp] VeloServe PHP Worker v{}", VERSION);
    println!("[vephp] PHP binary: {:?}", php_binary);
    println!("[vephp] Socket: {}", config.socket);
    println!("[vephp] Workers: {}", config.workers);
    println!("[vephp] Memory limit: {}", config.memory_limit);
    println!("[vephp] Timeout: {}s", config.max_execution_time);

    if let Some(ref user) = config.user {
        println!("[vephp] Running as user: {}", user);
    }

    let server = PhpWorkerServer::new(config, php_binary);

    if let Err(e) = server.run() {
        eprintln!("[vephp] Fatal error: {}", e);
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
