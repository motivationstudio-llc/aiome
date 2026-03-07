/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use serde::{Deserialize, Serialize};
use std::env;
use anyhow::Result;

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
            ollama_base_url: env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
            oracle_api_key: env::var("ORACLE_API_KEY").ok(),
        })
    }
}
