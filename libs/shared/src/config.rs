/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

/// AIOME Core Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiomeConfig {
    pub db_path: Option<String>,
    pub log_level: String,
    pub ollama_base_url: String,
    pub oracle_api_key: Option<String>,
}

impl AiomeConfig {
    pub fn load() -> Result<Self> {
        Ok(Self {
            db_path: env::var("DB_PATH").ok(),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            ollama_base_url: env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            oracle_api_key: env::var("ORACLE_API_KEY").ok(),
        })
    }
}
