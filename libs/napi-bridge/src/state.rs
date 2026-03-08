use std::sync::Arc;
use tokio::sync::OnceCell;
use infrastructure::job_queue::SqliteJobQueue;
use aiome_core::error::AiomeError;

static DB: OnceCell<Arc<SqliteJobQueue>> = OnceCell::const_new();
static IMMUNE: OnceCell<Arc<infrastructure::immune_system::AdaptiveImmuneSystem>> = OnceCell::const_new();

pub async fn get_db() -> Result<&'static Arc<SqliteJobQueue>, AiomeError> {
    DB.get_or_try_init(|| async {
        let db_path = std::env::var("AIOME_DB_PATH").unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());
        let queue = SqliteJobQueue::new(&db_path).await?;
        Ok(Arc::new(queue))
    }).await
}

pub async fn get_immune() -> Result<&'static Arc<infrastructure::immune_system::AdaptiveImmuneSystem>, AiomeError> {
    IMMUNE.get_or_try_init(|| async {
        let host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
        let provider = Arc::new(aiome_core::llm_provider::OllamaProvider::new(host, model));
        let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider);
        Ok(Arc::new(immune_system))
    }).await
}
