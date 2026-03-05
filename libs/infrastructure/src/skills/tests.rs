/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
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
        
        let result = manager.call_skill("hello_skill", "test_timeout", "", None).await;
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
        
        let result = manager.call_skill("hello_skill", "test_config", "", Some(configs)).await.expect("Execution failed");
        assert_eq!(result, "Key: SECRET_TOKEN_123");
    }
}
