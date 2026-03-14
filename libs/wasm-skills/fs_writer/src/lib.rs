/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

// WASM ゲストはホスト関数を unsafe で呼ぶ必要がある。
// Wasmtime サンドボックスにより安全性は保証される。
#[allow(unsafe_code)]
#![warn(missing_docs)]

use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[host_fn]
extern "ExtismHost" {
    fn host_write(payload: String) -> String;
}

#[derive(Serialize, Deserialize)]
struct WriteRequest {
    pub path: String,
    pub content: String,
}

#[allow(dead_code)]
#[derive(Serialize)]
struct WriteResponse {
    pub success: bool,
    pub path: String,
    pub error: Option<String>,
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    // We just parse to validate JSON, then pass the whole JSON string to the host
    let _req: WriteRequest = serde_json::from_str(&input)?;

    // Call the host function (Aiome OS Sentinel handles all security and IO)
    // The host function expects the raw JSON string because it needs both path and content.
    let result_json = unsafe { host_write(input)? };

    // Return the host's response directly
    Ok(result_json)
}
