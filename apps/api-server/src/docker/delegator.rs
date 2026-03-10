use std::process::Command;
use std::fs;
use tracing::{info, error, warn};
use uuid::Uuid;

/// Phase 17-C: Docker Agent "Shadow Worker" Delegation
/// This function executes untrusted or dependency-heavy tasks inside a disposable 
/// docker-agent container to isolate the host Aiome system.
pub async fn delegate_docker_worker(agent_yaml_content: &str, task_prompt: &str) -> String {
    let session_id = Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("aiome-delegation-{}", session_id));
    
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        return format!("Error: Failed to create delegation sandbox: {}", e);
    }

    let yaml_path = temp_dir.join("agent.yaml");
    if let Err(e) = fs::write(&yaml_path, agent_yaml_content) {
        return format!("Error: Failed to write agent config: {}", e);
    }

    info!("🐳 [DockerDelegator] Spinning up Shadow Worker for session: {}", session_id);

    // Call docker-agent (expected to be in PATH or symlinked)
    // Using --exec and --json for stable parsing (scanned from docker-agent repo)
    let output = tokio::task::spawn_blocking({
        let yaml_path = yaml_path.clone();
        let task_prompt = task_prompt.to_string();
        move || {
            Command::new("docker")
                .arg("agent")
                .arg("run")
                .arg("--exec")
                .arg("--json")
                .arg(yaml_path.to_string_lossy().as_ref())
                .arg(task_prompt)
                .output()
        }
    }).await;

    // Clean up temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    match output {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            
            if out.status.success() {
                info!("✅ [DockerDelegator] Worker session {} completed successfully", session_id);
                // In a production scenario, we would parse the JSON here
                format!("[Docker Delegation Result]\n{}", stdout)
            } else {
                warn!("🚨 [DockerDelegator] Worker session {} failed", session_id);
                // Phase 17-C: Feedback Loop for Karma Generation
                // We return the error so the AdaptiveImmuneSystem can learn from it.
                format!("[Docker Delegation ERROR]\nReason: Execution failed\nStderr: {}\nStdout: {}", stderr, stdout)
            }
        },
        Ok(Err(e)) => {
            error!("❌ [DockerDelegator] Execution error: {}", e);
            format!("Error: Failed to execute docker-agent. Ensure it is installed as a CLI plugin or binary. Details: {}", e)
        },
        Err(e) => {
            format!("Error: Task join error: {}", e)
        }
    }
}
