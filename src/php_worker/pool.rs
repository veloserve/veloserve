//! Worker Pool Management
//!
//! Manages a pool of PHP worker processes for handling concurrent requests.
//! Uses EA-PHP, CloudLinux alt-PHP, or system php-cgi as the execution engine.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use crate::protocol::{PhpRequest, PhpResponse};

pub struct PhpWorker {
    pub id: usize,
    pub process: Child,
    pub busy: bool,
}

pub struct WorkerPool {
    workers: Vec<PhpWorker>,
    max_workers: usize,
    memory_limit: String,
    max_execution_time: u32,
    php_ini: Option<PathBuf>,
    php_binary: PathBuf,
    request_queue: VecDeque<PhpRequest>,
}

impl WorkerPool {
    pub fn new(
        max_workers: usize,
        memory_limit: String,
        max_execution_time: u32,
        php_ini: Option<PathBuf>,
        php_binary: PathBuf,
    ) -> Self {
        let mut pool = Self {
            workers: Vec::with_capacity(max_workers),
            max_workers,
            memory_limit,
            max_execution_time,
            php_ini,
            php_binary,
            request_queue: VecDeque::new(),
        };

        pool.spawn_workers();
        pool
    }

    fn spawn_workers(&mut self) {
        for id in 0..self.max_workers {
            match self.spawn_worker(id) {
                Ok(worker) => {
                    self.workers.push(worker);
                }
                Err(e) => {
                    eprintln!("[vephp] Failed to spawn worker {}: {}", id, e);
                }
            }
        }
    }

    fn spawn_worker(&self, id: usize) -> Result<PhpWorker, Box<dyn std::error::Error>> {
        let mut cmd = Command::new(&self.php_binary);

        if let Some(ref ini) = self.php_ini {
            cmd.arg("-c").arg(ini);
        }

        cmd.arg("-d").arg(format!("memory_limit={}", self.memory_limit));
        cmd.arg("-d").arg(format!("max_execution_time={}", self.max_execution_time));
        cmd.arg("-q");

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

    pub fn execute(&mut self, request: &PhpRequest) -> PhpResponse {
        if let Some(worker) = self.workers.iter_mut().find(|w| !w.busy) {
            worker.busy = true;
            let result = self.run_php(request);
            worker.busy = false;
            result
        } else if self.request_queue.len() < 100 {
            self.request_queue.push_back(request.clone());
            PhpResponse::queued()
        } else {
            PhpResponse::error("Worker pool exhausted, request dropped")
        }
    }

    fn run_php(&self, request: &PhpRequest) -> PhpResponse {
        let mut cmd = Command::new(&self.php_binary);
        cmd.arg("-d").arg(format!("memory_limit={}", self.memory_limit));
        cmd.arg("-d").arg(format!("max_execution_time={}", self.max_execution_time));
        cmd.arg(&request.script_path);

        // Pass CGI environment variables
        for (key, value) in &request.server_vars {
            cmd.env(key, value);
        }

        let output = cmd.output();

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
                PhpResponse::error(&format!("Failed to execute PHP ({:?}): {}", self.php_binary, e))
            }
        }
    }

    pub fn status_json(&self) -> String {
        let total = self.workers.len();
        let busy = self.workers.iter().filter(|w| w.busy).count();
        let available = total - busy;
        let queued = self.request_queue.len();

        format!(
            "{{\"total_workers\":{},\"busy\":{},\"available\":{},\"queued\":{},\"php_binary\":\"{}\"}}",
            total, busy, available, queued,
            self.php_binary.display()
        )
    }

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
