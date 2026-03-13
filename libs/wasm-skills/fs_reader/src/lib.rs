/*
 * Aiome - fs_reader Skill (WASM/WASI)
 */

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct ReadRequest {
    pub path: String,
}

#[derive(Serialize)]
struct ReadResponse {
    pub content: String,
    pub error: Option<String>,
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: ReadRequest = serde_json::from_str(&input)?;

    // In Aiome OS, skills are typically jail-rooted to /mnt/workspace
    // The request 'path' is relative to that root.
    let full_path = Path::new("/mnt").join(&req.path);

    // Validate path to prevent directory traversal
    let canonical_path = match std::fs::canonicalize(&full_path) {
        Ok(p) => p,
        Err(e) => {
            let res = ReadResponse {
                content: String::new(),
                error: Some(format!("Invalid path: {}", e)),
            };
            return Ok(serde_json::to_string(&res)?);
        }
    };

    if !canonical_path.to_string_lossy().starts_with("/mnt") {
        let res = ReadResponse {
            content: String::new(),
            error: Some("Security Violation: Path traversal blocked.".into()),
        };
        return Ok(serde_json::to_string(&res)?);
    }

    match fs::read_to_string(&canonical_path) {
        Ok(content) => {
            let res = ReadResponse {
                content,
                error: None,
            };
            Ok(serde_json::to_string(&res)?)
        }
        Err(e) => {
            let res = ReadResponse {
                content: String::new(),
                error: Some(format!("Could not read file: {}", e)),
            };
            Ok(serde_json::to_string(&res)?)
        }
    }
}
