/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use serde::{Deserialize, Serialize};

/// [A-1] Docker Agent ↔ Karma Feedback Loop
/// Represents the result of an execution inside the Docker sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl DelegationResult {
    /// Determines if the execution was successful based on exit code.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// High-level classification of the failure type.
    pub fn failure_category(&self) -> FailureCategory {
        if self.is_success() {
            return FailureCategory::None;
        }

        // Common exit codes and stderr patterns
        if self.exit_code == 124 || self.stderr.contains("timeout") {
            FailureCategory::Timeout
        } else if self.exit_code == 137 || self.stderr.contains("OOM") || self.stderr.contains("Out of memory") {
            FailureCategory::Oom
        } else if self.stderr.contains("Module not found") || self.stderr.contains("ImportError") {
            FailureCategory::DependencyMissing
        } else if self.stderr.contains("SyntaxError") {
            FailureCategory::SyntaxError
        } else {
            FailureCategory::UnknownRuntime
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureCategory {
    None,
    Timeout,
    Oom,
    DependencyMissing,
    SyntaxError,
    UnknownRuntime,
}
