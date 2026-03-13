use infrastructure::skills::{UnverifiedSkill, WasmSkillManager};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Helper to set up a temporary environment representing the "Downloading" phase
fn setup_test_workspace(
    skill_name: &str,
    wasm_source: &Path,
    meta_json: &str,
) -> (WasmSkillManager, PathBuf) {
    let temp_dir = env::temp_dir().join(format!("aiome_mbt_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();

    // Copy wasm if it exists
    if wasm_source.exists() {
        fs::copy(wasm_source, temp_dir.join(format!("{}.wasm", skill_name))).unwrap();
    }

    // Write meta JSON
    fs::write(
        temp_dir.join(format!("{}.meta.json", skill_name)),
        meta_json,
    )
    .unwrap();

    let manager = WasmSkillManager::new(&temp_dir, &temp_dir).unwrap();
    (manager, temp_dir)
}

/// Trace 1: Downloading -> ManifestCheck -> DryRunQuarantine -> Active
/// Expectation: The skill succeeds the Extism quarantine simulation and is promoted to VerifiedSkill.
#[tokio::test]
async fn test_mbt_quarantine_happy_path() {
    // Generate a minimalist WASM byte array that exports "execute"
    // This removes external dependency on workspace/skills and ensures test determinism.
    // Equivalent to `(module (func (export "execute")))`
    let min_wasm: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x0b, 0x01, 0x07, 0x65, 0x78, 0x65, 0x63, 0x75, 0x74, 0x65, 0x00,
        0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ];

    let temp_dir = env::temp_dir().join(format!("aiome_mbt_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let wasm_path = temp_dir.join("StubSkill.wasm");
    fs::write(&wasm_path, min_wasm).unwrap();

    let safe_meta = r#"{
        "name": "StubSkill",
        "description": "Safe test skill",
        "capabilities": ["execute"],
        "inputs": ["String"],
        "outputs": ["String"],
        "allowed_hosts": [],
        "permissions": {
            "allow_network": false,
            "allow_filesystem_write": false,
            "allow_shell_execution": false,
            "allowed_domains": []
        }
    }"#;
    fs::write(temp_dir.join("StubSkill.meta.json"), safe_meta).unwrap();

    let manager = WasmSkillManager::new(&temp_dir, &temp_dir).unwrap();

    let unverified = UnverifiedSkill {
        name: "StubSkill".to_string(),
        input_test_payload: "HelloWorld".to_string(),
    };

    let result = unverified.verify(&manager).await;

    // Assert state transition to "Active" (VerifiedSkill type)
    assert!(
        result.is_ok(),
        "Skill should reach Active state (VerifiedSkill) after dry-run simulation. Found: {:?}",
        result
    );

    let verified_skill = result.unwrap();
    assert_eq!(verified_skill.name(), "StubSkill");
}

/// Trace 2: ManifestCheck -> DryRunQuarantine -> Violated
/// Expectation: A skill that triggers an error in the Extism plugin (OOM, missing exports) or is missing is rejected.
#[tokio::test]
async fn test_mbt_quarantine_dry_run_fails() {
    let (manager, _dir) = setup_test_workspace("missing_skill", &PathBuf::new(), "{}");

    let unverified = UnverifiedSkill {
        name: "missing_skill".to_string(),
        input_test_payload: "test".to_string(),
    };

    let result = unverified.verify(&manager).await;

    // Assert state transition to "Violated" (Err)
    assert!(
        result.is_err(),
        "Skill should reach Violated state and fail verification"
    );
}

/// Trace 3: Database DbC Contract Checks
/// Expectation: Precondition failures (e.g. payload too large) trigger DbC assertions
#[tokio::test]
#[should_panic(expected = "Payload limits exceeded")]
async fn test_mbt_quarantine_payload_violation() {
    let (manager, _dir) = setup_test_workspace("hello_skill", &PathBuf::new(), "{}");

    // Creating a payload larger than 50,000 bytes
    let massive_string = "A".repeat(50_001);

    let unverified = UnverifiedSkill {
        name: "hello_skill".to_string(),
        input_test_payload: massive_string,
    };

    // This should panic due to #[requires(self.input_test_payload.len() < 50_000)]
    let _ = unverified.verify(&manager).await;
}
