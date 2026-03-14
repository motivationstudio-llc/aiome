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

#[cfg(not(any(target_os = "android", target_os = "ios")))]

use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[host_fn]
extern "ExtismHost" {
    fn host_exec(cmd: String) -> String;
}

#[derive(Deserialize)]
struct ExecRequest {
    pub cmd: String,
}

#[derive(Serialize)]
struct ExecResponse {
    pub stdout: String,
    pub stderr: Option<String>,
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: ExecRequest = serde_json::from_str(&input)?;

    // Call the host function (Aiome OS Sentinel)
    let result = unsafe { host_exec(req.cmd)? };

    let res = ExecResponse {
        stdout: result,
        stderr: None,
    };
    Ok(serde_json::to_string(&res)?)
}
