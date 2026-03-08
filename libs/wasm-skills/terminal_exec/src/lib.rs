/*
 * Aiome - terminal_exec Skill (WASM/WASI)
 */

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
