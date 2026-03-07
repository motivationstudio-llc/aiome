/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use std::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::Mutex;
use sysinfo::{System, Pid};
use tracing::{info, warn, error};
use std::time::Duration;
use tokio::time::sleep;

/// サイドカー・プロセスの管理を司る構造体 ("The Reaper")
pub struct SidecarManager {
    /// 管理下の子プロセス
    child: Arc<Mutex<Option<Child>>>,
    /// 許可されたプロセス名のリスト
    allowed_names: Vec<String>,
}

impl SidecarManager {
    pub fn new(allowed_names: Vec<String>) -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            allowed_names,
        }
    }

    /// ポートを占有しているプロセスを特定し、許可リストにある場合のみクリーンアップする
    pub async fn clean_port(&self, port: u16) -> anyhow::Result<()> {
        info!("🔍 SidecarManager: Cleaning port {}...", port);

        // macOS では lsof -i :<port> -t を使用して PID を取得するのが確実
        let output = Command::new("lsof")
            .arg("-i")
            .arg(format!(":{}", port))
            .arg("-t")
            .output()?;

        let pid_str = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<&str> = pid_str.lines().collect();

        if pids.is_empty() {
            info!("✅ SidecarManager: Port {} is already free.", port);
            return Ok(());
        }

        let mut sys = System::new_all();
        sys.refresh_all();

        for pid_str in pids {
            if let Ok(pid_val) = pid_str.parse::<usize>() {
                let pid = Pid::from(pid_val);
                if let Some(process) = sys.process(pid) {
                    let name = process.name();
                    
                    // RA-01: 許可リストによる身元確認
                    let is_allowed = self.allowed_names.iter().any(|allowed| name.contains(allowed));
                    
                    if is_allowed {
                        warn!("⚠️  SidecarManager: Killing allowed process '{}' (PID: {}) on port {}", name, pid, port);
                        self.graceful_kill(pid).await;
                    } else {
                        error!("⛔ SidecarManager: SAFETY VIOLATION! Unknown process '{}' (PID: {}) is occupying port {}. Skipping to avoid system damage.", name, pid, port);
                        return Err(anyhow::anyhow!("Port {} is occupied by an unauthorized process: {}", port, name));
                    }
                }
            }
        }

        Ok(())
    }

    /// プロセスとそのグループを安全に終了させる (Graceful-then-Hard Group Kill)
    async fn graceful_kill(&self, pid: Pid) {
        let pid_val = pid.as_u32() as i32;
        
        // 1. SIGTERM (プロセスグループ全体に送信)
        info!("📩 SidecarManager: Sending SIGTERM to Process Group {}...", pid);
        unsafe {
            // -pid はプロセスグループ全体を対象とする
            libc::kill(-pid_val, libc::SIGTERM);
        }

        // 2. 猶予期間 (3秒)
        sleep(Duration::from_secs(3)).await;

        // 3. プロセス生存確認と SIGKILL (グループ全体)
        let mut sys = System::new_all();
        sys.refresh_process(pid);
        
        if sys.process(pid).is_some() {
            warn!("💢 SidecarManager: Process Group {} did not exit. Sending SIGKILL to group...", pid);
            unsafe {
                libc::kill(-pid_val, libc::SIGKILL);
            }
        } else {
            info!("🆗 SidecarManager: Process Group {} exited gracefully.", pid);
        }
    }

    /// サイドカープロセスを開始する
    pub async fn spawn(&self, mut command: Command) -> anyhow::Result<()> {
        info!("🚀 SidecarManager: Spawning sidecar process...");
        
        // プロセスグループを分離して、ゾンビ化を防ぐ
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }

        let child = command.spawn()?;
        let mut guard = self.child.lock().await;
        *guard = Some(child);
        
        Ok(())
    }
}

/// RA-02: 道連れ終了 (Drop Trait)
impl Drop for SidecarManager {
    fn drop(&mut self) {
        // Drop は 同期的なので、ここではブロッキングな終了処理を行う
        let mut guard = match self.child.try_lock() {
            Ok(g) => g,
            Err(_) => {
                error!("❌ SidecarManager: Could not lock child process during drop!");
                return;
            }
        };

        if let Some(mut child) = guard.take() {
            let pid = child.id() as i32;
            warn!("💀 SidecarManager: Main process exiting. Killing sidecar group (PGID: {})...", pid);
            
            // 同期的な SIGTERM (グループ全体)
            unsafe {
                libc::kill(-pid, libc::SIGTERM);
            }
            
            // 簡易的な待機 (1秒)
            std::thread::sleep(Duration::from_secs(1));
            
            // 最終的な SIGKILL (グループ全体)
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
            
            let _ = child.wait();
            info!("⚰️  SidecarManager: Sidecar group PGID {} reaped.", pid);
        }
    }
}
