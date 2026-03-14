/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::error::AiomeError;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub allowed_binaries: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_binaries: vec![
                "ls".to_string(),
                "cat".to_string(),
                "cargo".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "wc".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "git".to_string(),
                "rustc".to_string(),
                "node".to_string(),
                "npm".to_string(),
                "python3".to_string(),
                "mkdir".to_string(),
                "cp".to_string(),
                "mv".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "diff".to_string(),
                "tree".to_string(),
                "which".to_string(),
                "env".to_string(),
            ],
        }
    }
}

impl SecurityConfig {
    pub fn load_or_default() -> Self {
        let path = std::path::Path::new("workspace/config/security.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<SecurityConfig>(&content) {
                    info!(
                        "🛡️ [SecurityConfig] Loaded whitelist from workspace/config/security.json."
                    );
                    return config;
                }
            }
        }
        Self::default()
    }
}

pub static GLOBAL_SECURITY_CONFIG: once_cell::sync::Lazy<SecurityConfig> =
    once_cell::sync::Lazy::new(SecurityConfig::load_or_default);

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

    /// シェルコマンドの実行を検証し、許可されていれば実行する (同期版)
    pub fn safe_exec(&self, cmd_str: &str) -> Result<String, AiomeError> {
        info!("🛡️ [BastionGuard] 検証中: {}", cmd_str);

        // 1. マニフェスト・チェック
        if !self.manifest.allow_shell_execution {
            error!("🚨 [SECURITY VIOLATION] Shell execution is disabled.");
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Forbidden.".to_string(),
            });
        }

        // 2. インジェクション・フィルタ
        let dangerous_parts = [";", "&&", "||", ">", "<", "|", "`", "$("];
        for part in dangerous_parts {
            if cmd_str.contains(part) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Security Violation: '{}' prohibited.", part),
                });
            }
        }

        // 3. センシティブなパス
        if cmd_str.contains("/etc/") || cmd_str.contains("~/.ssh") || cmd_str.contains(".env") {
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Sensitive access.".to_string(),
            });
        }

        info!("✅ [BastionGuard] 検証完了。コマンドを実行します...");

        // 4. Safer Execution: Use direct binary execution if possible to avoid terminal injection
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Empty command.".into(),
            });
        }
        let binary = parts[0];
        let args = &parts[1..];

        // Strict Whitelist check against SecurityConfig (Global Singleton)
        if !GLOBAL_SECURITY_CONFIG
            .allowed_binaries
            .contains(&binary.to_string())
        {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Security Violation: Binary '{}' is not in the whitelist.",
                    binary
                ),
            });
        }

        use std::process::Command;

        let output =
            Command::new(binary)
                .args(args)
                .output()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Execution failed: {}", e),
                })?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(AiomeError::Infrastructure {
                reason: format!("Command error: {}", err_msg),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
