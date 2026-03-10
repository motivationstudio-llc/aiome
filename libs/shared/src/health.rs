/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use sysinfo::{System, Pid};
use serde::{Deserialize, Serialize};
use std::fmt;

/// 秘密情報をログ出力から保護するためのラッパー
#[derive(Clone, Deserialize, Serialize)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }

    pub fn expose(&self) -> &T {
        &self.0
    }
}

// 誤ってログに出力されないようにマスクする
impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "********")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "********")
    }
}

/// リソースの使用状況
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    pub memory_usage_mb: u64,
    pub total_memory_mb: u64,
    pub cpu_usage_percent: f32,
    pub vram_usage_mb: Option<u64>,
    pub disk_free_gb: u64,
    pub total_disk_gb: u64,
    pub open_files: Option<u64>,
}

/// システムの状態を監視する
pub struct HealthMonitor {
    sys: System,
    pid: Pid,
    disks: sysinfo::Disks,
}

impl HealthMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let pid = Pid::from(std::process::id() as usize);
        Self { sys, pid, disks }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMonitor {
    pub fn check(&mut self) -> ResourceStatus {
        // 全体のメモリと特定のプロセスをリフレッシュ
        self.sys.refresh_memory();
        self.sys.refresh_process(self.pid);
        self.disks.refresh_list();
        
        let mut memory_usage_mb = 0;
        let mut cpu_usage_percent = 0.0;
        let total_memory_mb = self.sys.total_memory() / 1024 / 1024;
        
        if let Some(process) = self.sys.process(self.pid) {
            memory_usage_mb = process.memory() / 1024 / 1024;
            cpu_usage_percent = process.cpu_usage();
        }

        // ルートディレクトリの空き容量を取得
        let disk_info = self.disks.iter()
            .find(|d| d.mount_point() == std::path::Path::new("/"))
            .map(|d| (d.available_space() / 1024 / 1024 / 1024, d.total_space() / 1024 / 1024 / 1024))
            .unwrap_or((0, 0));

        ResourceStatus {
            memory_usage_mb,
            total_memory_mb,
            cpu_usage_percent,
            vram_usage_mb: None, // TODO: macOS/Linux GPU monitoring
            disk_free_gb: disk_info.0,
            total_disk_gb: disk_info.1,
            open_files: None,
        }
    }
}
