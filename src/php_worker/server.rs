//! vephp Worker Server
//!
//! Manages the socket server that receives PHP requests from VeloServe
//! and dispatches them to worker processes using EA-PHP or system PHP.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::Config;
use crate::pool::WorkerPool;
use crate::protocol::{PhpRequest, PhpResponse, RequestType};

pub struct PhpWorkerServer {
    config: Config,
    pool: Arc<Mutex<WorkerPool>>,
}

impl PhpWorkerServer {
    pub fn new(config: Config, php_binary: PathBuf) -> Self {
        let pool = Arc::new(Mutex::new(WorkerPool::new(
            config.workers,
            config.memory_limit.clone(),
            config.max_execution_time,
            config.php_ini.clone(),
            php_binary,
        )));

        Self { config, pool }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.socket.starts_with('/') {
            let _ = std::fs::remove_file(&self.config.socket);

            // Ensure parent directory exists
            if let Some(parent) = std::path::Path::new(&self.config.socket).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let listener = UnixListener::bind(&self.config.socket)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&self.config.socket)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o666);
            std::fs::set_permissions(&self.config.socket, permissions)?;
        }

        println!("[vephp] Listening on: {}", self.config.socket);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let pool = Arc::clone(&self.pool);
                    let verbose = self.config.verbose;

                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, pool, verbose) {
                            eprintln!("[vephp] Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[vephp] Accept error: {}", e);
                }
            }
        }

        Ok(())
    }
}

fn handle_connection(
    mut stream: UnixStream,
    pool: Arc<Mutex<WorkerPool>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 65536];
    let bytes_read = stream.read(&mut buffer)?;

    if bytes_read == 0 {
        return Ok(());
    }

    let request: PhpRequest = match bincode::deserialize(&buffer[..bytes_read]) {
        Ok(req) => req,
        Err(e) => {
            let response = PhpResponse::error(&format!("Invalid request: {}", e));
            send_response(&mut stream, &response)?;
            return Ok(());
        }
    };

    if verbose {
        println!("[vephp] Request: {:?} {}",
            request.request_type,
            request.script_path.display()
        );
    }

    let response = match request.request_type {
        RequestType::Execute => {
            let mut pool = pool.lock().unwrap();
            pool.execute(&request)
        }
        RequestType::HealthCheck => {
            PhpResponse::ok("healthy", "")
        }
        RequestType::Status => {
            let pool = pool.lock().unwrap();
            PhpResponse::ok("status", &pool.status_json())
        }
    };

    send_response(&mut stream, &response)?;

    Ok(())
}

fn send_response(
    stream: &mut UnixStream,
    response: &PhpResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    let response_bytes = bincode::serialize(response)?;
    stream.write_all(&response_bytes)?;
    stream.flush()?;
    Ok(())
}
