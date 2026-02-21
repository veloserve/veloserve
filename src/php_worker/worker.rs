//! Individual PHP Worker
//!
//! Manages a single PHP worker process and communication with it.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::protocol::{PhpRequest, PhpResponse};

/// Individual PHP worker process
pub struct Worker {
    id: usize,
    process: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl Worker {
    /// Spawn a new PHP worker process
    pub fn spawn(id: usize, php_ini: Option<&PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut cmd = Command::new("php");
        
        // Add custom php.ini if provided
        if let Some(ini) = php_ini {
            cmd.arg("-c").arg(ini);
        }
        
        // Enable CGI mode
        cmd.arg("-q"); // Quiet mode
        
        // Set up pipes for communication
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut process = cmd.spawn()?;
        let stdin = process.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = process.stdout.take().ok_or("Failed to get stdout")?;

        Ok(Self {
            id,
            process,
            stdin,
            stdout,
        })
    }

    /// Execute a PHP request in this worker
    pub fn execute(&mut self, request: &PhpRequest) -> Result<PhpResponse, Box<dyn std::error::Error>> {
        // Serialize request
        let request_bytes = bincode::serialize(request)?;
        
        // Send to worker
        self.stdin.write_all(&request_bytes)?;
        self.stdin.flush()?;

        // Read response
        let mut buffer = vec![0u8; 65536];
        let bytes_read = self.stdout.read(&mut buffer)?;
        
        // Deserialize response
        let response: PhpResponse = bincode::deserialize(&buffer[..bytes_read])?;
        
        Ok(response)
    }

    /// Check if worker is still alive
    pub fn is_alive(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(None) => true,  // Still running
            _ => false,        // Exited or error
        }
    }

    /// Get worker ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Kill the worker process
    pub fn kill(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.process.kill()?;
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_spawn() {
        // This test requires PHP to be installed
        if let Ok(mut worker) = Worker::spawn(0, None) {
            assert!(worker.is_alive());
            let _ = worker.kill();
        }
    }
}
