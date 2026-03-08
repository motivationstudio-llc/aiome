/*
 * Aiome - fs_writer Skill (WASM/WASI)
 */

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

#[derive(Serialize)]
struct WriteResponse {
    pub success: bool,
    pub path: String,
    pub error: Option<String>,
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    // We just parse to validate JSON, then pass the whole JSON string to the host
    let req: WriteRequest = serde_json::from_str(&input)?;
    
    // Call the host function (Aiome OS Sentinel handles all security and IO)
    // The host function expects the raw JSON string because it needs both path and content.
    let result_json = unsafe { host_write(input)? };
    
    // Return the host's response directly
    Ok(result_json)
}
