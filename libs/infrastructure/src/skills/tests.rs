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
        root.pop();
        root.pop();
        let skills_dir = root.join("workspace/skills");
        let manager = WasmSkillManager::new(&skills_dir, &root)
            .expect("Failed to create manager")
            .with_limits(1024 * 1024, std::time::Duration::from_millis(500));

        let verified = crate::skills::VerifiedSkill::promote("hello_skill".to_string());
        let result = manager
            .call_skill(&verified, "test_timeout", "", None)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_wasm_skill_config_injection() {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.pop();
        root.pop();
        let skills_dir = root.join("workspace/skills");
        let manager = WasmSkillManager::new(&skills_dir, &root).expect("Failed to create manager");

        let mut configs = std::collections::HashMap::new();
        configs.insert("api_key".to_string(), "SECRET_TOKEN_123".to_string());

        let verified = crate::skills::VerifiedSkill::promote("hello_skill".to_string());
        let result = manager
            .call_skill(&verified, "test_config", "", Some(configs))
            .await
            .expect("Execution failed");
        assert_eq!(result, "Key: SECRET_TOKEN_123");
    }

    #[tokio::test]
    async fn test_dry_run_call_validation() {
        let mut root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.pop();
        root.pop();
        let skills_dir = root.join("workspace/skills");
        let manager = WasmSkillManager::new(&skills_dir, &root).expect("Failed to create manager");

        // Dry-run should at least execute without system-level error.
        // Whether it returns true or false depends on the skill's specific behavior
        // when running without its actual configuration (config injection is disabled in dry-run).
        let result = manager.dry_run_skill("hello_skill", "{}").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dry_run_missing_skill_error() {
        let mut root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.pop();
        root.pop();
        let skills_dir = root.join("workspace/skills");
        let manager = WasmSkillManager::new(&skills_dir, &root).expect("Failed to create manager");

        let result = manager.dry_run_skill("non_existent_skill", "{}").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_hot_reload_skills() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // Use temp_dir as root as well for the manager
        let skills_dir_buf = skills_dir.to_path_buf();
        let manager =
            WasmSkillManager::new(&skills_dir_buf, &temp_dir.path().to_path_buf()).unwrap();

        // No skills initially
        assert!(manager.list_skills().is_empty());

        // Add a fake wasm
        std::fs::write(skills_dir.join("test.wasm"), b"wasm").unwrap();

        // list_skills should find it
        let skills = manager.list_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "test");

        // hot_reload_skills should return it
        let reloaded = manager.hot_reload_skills();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0], "test");
    }

    #[tokio::test]
    async fn test_ensure_forge_workspace() {
        let temp_dir = tempfile::tempdir().unwrap();
        let template_dir = temp_dir.path().join("template");
        let output_dir = temp_dir.path().join("output");

        let forge = crate::skills::forge::SkillForge::new(&template_dir, &output_dir);
        forge.ensure_forge_workspace().unwrap();

        assert!(template_dir.join("Cargo.toml").exists());
        assert!(template_dir.join("src/lib.rs").exists());

        let cargo_contents = std::fs::read_to_string(template_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_contents.contains("extism-pdk"));
    }

    #[test]
    fn test_generate_seatbelt_profile() {
        // Mock paths for testing profile generation logic
        let temp_dir = std::path::PathBuf::from("/tmp/aiome_test");
        let output_dir = std::path::PathBuf::from("/tmp/aiome_output");
        let forge = crate::skills::forge::SkillForge::new(&temp_dir, &output_dir);

        let build_dir = std::path::PathBuf::from("/tmp/build");
        let profile = forge.generate_seatbelt_profile(&build_dir);

        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(allow default)"));
        assert!(profile.contains(&build_dir.to_string_lossy().to_string()));
    }
}
