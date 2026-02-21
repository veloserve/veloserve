//! Worker Pool Management
//!
//! Manages a pool of PHP worker processes for handling concurrent requests.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::protocol::{PhpRequest, PhpResponse};

/// Represents a PHP worker process
pub struct PhpWorker {
    pub id: usize,
    pub process: Child,
    pub busy: bool,
}

/// Pool of PHP worker processes
pub struct WorkerPool {
    workers: Vec<PhpWorker>,
    max_workers: usize,
    memory_limit: String,
    max_execution_time: u32,
    php_ini: Option<PathBuf>,
    request_queue: VecDeque<PhpRequest>,
}

impl WorkerPool {
    pub fn new(
        max_workers: usize,
        memory_limit: String,
        max_execution_time: u32,
        php_ini: Option<PathBuf>,
    ) -> Self {
        let mut pool = Self {
            workers: Vec::with_capacity(max_workers),
            max_workers,
            memory_limit,
            max_execution_time,
            php_ini,
            request_queue: VecDeque::new(),
        };

        // Initialize workers
        pool.spawn_workers();
        
        pool
    }

    /// Spawn initial worker processes
    fn spawn_workers(&mut self) {
        for id in 0..self.max_workers {
            match self.spawn_worker(id) {
                Ok(worker) => {
                    self.workers.push(worker);
                }
                Err(e) => {
                    eprintln!("[veloserve-php] Failed to spawn worker {}: {}", id, e);
                }
            }
        }
    }

    /// Spawn a single PHP worker process
    fn spawn_worker(&self, id: usize) -> Result<PhpWorker, Box<dyn std::error::Error>> {
        // Build PHP command
        let mut cmd = Command::new("php");
        
        // Add PHP ini if specified
        if let Some(ref ini) = self.php_ini {
            cmd.arg("-c").arg(ini);
        }
        
        // Set PHP settings
        cmd.arg("-d").arg(format!("memory_limit={}", self.memory_limit));
        cmd.arg("-d").arg(format!("max_execution_time={}", self.max_execution_time));
        
        // Run PHP in CGI mode for now (will be replaced with embedded SAPI later)
        cmd.arg("-q"); // Quiet mode
        
        // Redirect stdin/stdout
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let process = cmd.spawn()?;

        Ok(PhpWorker {
            id,
            process,
            busy: false,
        })
    }

    /// Execute a PHP request using an available worker
    pub fn execute(&mut self, request: &PhpRequest) -> PhpResponse {
        // Find available worker
        if let Some(worker) = self.workers.iter_mut().find(|w| !w.busy) {
            worker.busy = true;
            
            // Execute PHP script
            let result = self.run_php(worker, request);
            
            worker.busy = false;
            result
        } else {
            // No available workers - queue or error
            if self.request_queue.len() < 100 {
                self.request_queue.push_back(request.clone());
                PhpResponse::queued()
            } else {
                PhpResponse::error("Worker pool exhausted, request dropped")
            }
        }
    }

    /// Run PHP script in worker
    fn run_php(&self, worker: &mut PhpWorker, request: &PhpRequest) -> PhpResponse {
        // For MVP: use system php command
        // In production: use embedded PHP SAPI via FFI
        
        let output = std::process::Command::new("php")
            .arg("-d").arg(format!("memory_limit={}", self.memory_limit))
            .arg("-d").arg(format!("max_execution_time={}", self.max_execution_time))
            .arg(&request.script_path)
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                
                if result.status.success() {
                    PhpResponse::ok(&stdout, &stderr)
                } else {
                    PhpResponse::error(&format!("PHP exit code {:?}: {}", 
                        result.status.code(), stderr))
                }
            }
            Err(e) => {
                PhpResponse::error(&format!("Failed to execute PHP: {}", e))
            }
        }
    }

    /// Get pool status as JSON
    pub fn status_json(&self) -> String {
        let total = self.workers.len();
        let busy = self.workers.iter().filter(|w| w.busy).count();
        let available = total - busy;
        let queued = self.request_queue.len();

        format!(
            "{{\"total_workers\":{},\"busy\":{},\"available\":{},\"queued\":{}}}",
            total, busy, available, queued
        )
    }

    /// Shutdown all workers
    pub fn shutdown(&mut self) {
        for worker in &mut self.workers {
            let _ = worker.process.kill();
        }
        self.workers.clear();
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}
