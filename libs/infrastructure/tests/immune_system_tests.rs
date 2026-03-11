use infrastructure::immune_system::AdaptiveImmuneSystem;
use infrastructure::job_queue::SqliteJobQueue;
use aiome_core::traits::{JobQueue, JobStatus, KarmaEntry, KarmaSearchResult};
use aiome_core::llm_provider::LlmProvider;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Debug, Clone)]
struct MockLlmProvider {
    json_response: String,
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(&self, _prompt: &str, _system: Option<&str>) -> Result<String, AiomeError> {
        Ok(self.json_response.clone())
    }
    fn name(&self) -> &str { "Mock" }
}

async fn create_test_queue() -> (SqliteJobQueue, TempDir) {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");
    let db_path_str = db_path.to_str().expect("Invalid path");
    let jq = SqliteJobQueue::new(&format!("sqlite://{}", db_path_str)).await.expect("Failed to create test job queue");
    (jq, tmp_dir)
}

#[tokio::test]
async fn test_verify_intent_baseline() {
    let mock_provider = Arc::new(MockLlmProvider { json_response: "".to_string() });
    let immune_system = AdaptiveImmuneSystem::new(mock_provider);
    
    let (jq, _tmp) = create_test_queue().await;
    
    // Baseline detection should block "rm -rf /"
    let baseline_result = immune_system.verify_intent("Try to rm -rf / please", &jq).await.unwrap();
    assert!(baseline_result.is_some());
    let rule = baseline_result.unwrap();
    assert_eq!(rule.action, "Block");
    assert_eq!(rule.id, "sentinel-baseline");
    
    // Normal text should pass
    let safe_result = immune_system.verify_intent("Just a normal friendly chat", &jq).await.unwrap();
    assert!(safe_result.is_none());
}

#[tokio::test]
async fn test_analyze_threats_and_verify() {
    let mock_response = r#"
    {
        "pattern": "select \\* from users",
        "severity": 90,
        "action": "Block"
    }
    "#.to_string();

    let mock_provider = Arc::new(MockLlmProvider { json_response: mock_response });
    let immune_system = AdaptiveImmuneSystem::new(mock_provider.clone());
    
    let (jq, _tmp) = create_test_queue().await;
    
    // Add a dummy job and karma to trigger the fetch_relevant_karma in analyze_threats
    let job_id = jq.enqueue("Task", "Security Test", "Style", None).await.unwrap();
    jq.store_karma(&job_id, "global", "security threat injection error", "Technical", "hash-1", None, None).await.unwrap();

    let new_rules_count = immune_system.analyze_threats(&jq).await.unwrap();
    assert_eq!(new_rules_count, 1);

    // Now verification should catch the newly generated rule
    let result = immune_system.verify_intent("Please select * from users", &jq).await.unwrap();
    assert!(result.is_some());
    let rule = result.unwrap();
    assert_eq!(rule.pattern, "select \\* from users");
    assert_eq!(rule.action, "Block");
    assert_eq!(rule.severity, 90);
}
