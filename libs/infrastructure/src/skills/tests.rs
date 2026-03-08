/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

#[cfg(test)]
mod tests {
    use crate::skills::WasmSkillManager;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_wasm_skill_timeout() {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.pop(); root.pop();
        let skills_dir = root.join("workspace/skills");
        let manager = WasmSkillManager::new(&skills_dir, &root).expect("Failed to create manager")
                      .with_limits(1024*1024, std::time::Duration::from_millis(500));
        
        let verified = crate::skills::VerifiedSkill::promote("hello_skill".to_string());
        let result = manager.call_skill(&verified, "test_timeout", "", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_wasm_skill_config_injection() {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.pop(); root.pop();
        let skills_dir = root.join("workspace/skills");
        let manager = WasmSkillManager::new(&skills_dir, &root).expect("Failed to create manager");
        
        let mut configs = std::collections::HashMap::new();
        configs.insert("api_key".to_string(), "SECRET_TOKEN_123".to_string());
        
        let verified = crate::skills::VerifiedSkill::promote("hello_skill".to_string());
        let result = manager.call_skill(&verified, "test_config", "", Some(configs)).await.expect("Execution failed");
        assert_eq!(result, "Key: SECRET_TOKEN_123");
    }
}
