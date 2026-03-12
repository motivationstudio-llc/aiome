/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expression {
    pub id: String,
    pub content: String,        // 生成されたテキスト
    pub emotion: String,        // 推定された感情 ("curious", "reflective", "excited", etc.)
    pub karma_refs: Vec<String>, // 参照したKarmaのID (JSON array serialized in DB)
    pub created_at: String,
}

pub mod engine;
