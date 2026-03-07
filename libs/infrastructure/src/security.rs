/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use aiome_core::error::AiomeError;
use tracing::{info, error};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionManifest {
    pub allow_network: bool,
    pub allow_filesystem_write: bool,
    pub allow_shell_execution: bool,
    pub allowed_domains: Vec<String>,
}

impl Default for PermissionManifest {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_filesystem_write: false,
            allow_shell_execution: false,
            allowed_domains: vec![],
        }
    }
}

/// Phase 2: Runtime Enforcement (The Bastion Guard)
/// 
/// エージェントが実行しようとする「アクション」を監視し、
/// 権限マニフェストおよびOSレベルの制限（seccomp等）と照合する。
pub struct BastionGuard {
    manifest: PermissionManifest,
}

impl BastionGuard {
    pub fn new(manifest: PermissionManifest) -> Self {
        Self { manifest }
    }

    /// シェルコマンドの実行を検証し、許可されていれば実行する
    pub fn safe_exec(&self, cmd_str: &str) -> Result<String, AiomeError> {
        info!("🛡️ [BastionGuard] 検証中: {}", cmd_str);

        // 1. マニフェスト・チェック (第2層-A)
        if !self.manifest.allow_shell_execution {
            error!("🚨 [SECURITY VIOLATION] Shell execution is disabled in manifest.");
            return Err(AiomeError::Infrastructure { reason: "Security Violation: Shell execution is forbidden.".to_string() });
        }

        // 2. インジェクション・フィルタ (第2層-B)
        let dangerous_parts = [";", "&&", "||", ">", "<", "|", "`", "$("];
        for part in dangerous_parts {
            if cmd_str.contains(part) {
                error!("🚨 [SECURITY VIOLATION] Malicious command chaining detected: {}", part);
                return Err(AiomeError::Infrastructure { reason: format!("Security Violation: Use of '{}' is prohibited in input.", part) });
            }
        }

        // 3. OSレベルの制限 (第2層-C: Seccomp/Sandbox)
        // NOTE: macOS の場合は sandbox-exec 等が候補。Linux の場合は seccomp-bpf。
        // ここではデモ用に、特定の「保護されたディレクトリ」へのアクセスをチェックする。
        if cmd_str.contains("/etc/") || cmd_str.contains("~/.ssh") || cmd_str.contains(".env") {
            error!("🚨 [SECURITY VIOLATION] Attempted access to sensitive OS region: {}", cmd_str);
            return Err(AiomeError::Infrastructure { reason: "Security Violation: Access to sensitive system files is blocked.".to_string() });
        }

        info!("✅ [BastionGuard] 検証完了。コマンドを実行します...");

        #[cfg(target_os = "linux")]
        {
             // Linux環境ならここで seccomp-bpf を適用した子プロセスを生成
             info!("(Linux-specific seccomp/pledge would be applied here)");
        }

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", cmd_str]).output()
        } else {
            Command::new("sh").args(["-c", cmd_str]).output()
        }.map_err(|e| AiomeError::Infrastructure { reason: format!("Execution failed: {}", e) })?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(AiomeError::Infrastructure { reason: format!("Command exited with error: {}", err_msg) });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
