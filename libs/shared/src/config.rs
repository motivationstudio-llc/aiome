/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
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
