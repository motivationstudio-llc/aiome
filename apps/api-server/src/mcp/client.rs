use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};
use anyhow::Result;

/// Phase 17-B: Zombie Defense - Managed child process that kills its entire group on drop.
pub struct ManagedChild {
    pub child: Child,
    pub id: String,
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        let pid = self.child.id();
        info!("🧹 [MCP] Dropping ManagedChild ({}). Cleaning up PGID...", self.id);
        let _ = signal::kill(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);
    }
}

pub struct McpProcessManager {
    processes: Arc<Mutex<HashMap<String, ManagedChild>>>,
}

impl McpProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn_stdio_server(&self, id: String, cmd: &str, args: Vec<String>) -> Result<()> {
        info!("🚀 [MCP] Spawning stdio server: {} (PGID defense enabled)", cmd);
        
        let child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0) // Assign to new process group for collective cleanup
            .spawn()?;

        let mut procs = self.processes.lock().await;
        procs.insert(id.clone(), ManagedChild { child, id });
        Ok(())
    }

    pub async fn cleanup(&self, id: &str) {
        let mut procs = self.processes.lock().await;
        if procs.remove(id).is_some() {
            info!("🗑️ [MCP] Explicitly cleaned up session: {}", id);
        }
    }

    pub async fn kill_all(&self) {
        let mut procs = self.processes.lock().await;
        info!("💥 [MCP] Killing all {} managed MCP child processes", procs.len());
        procs.clear(); // Eviction triggers Drop -> SIGKILL to PGID
    }
}

// In the next step, we will implement the actual JSON-RPC client logic 
// that uses these managed processes.
