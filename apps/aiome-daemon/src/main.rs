use anyhow::Result;
use dotenvy::dotenv;
use aiome_core::contracts::ConceptRequest;
use aiome_core::traits::JobQueue;
use infrastructure::job_queue::SqliteJobQueue;
use shared::config::AiomeConfig;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("🚀 Starting aiome-daemon (OSS Edition)");

    let config = AiomeConfig::load()?;
    let db_path = config.db_path.clone().unwrap_or_else(|| "workspace/aiome.db".to_string());
    info!("📂 Using database: {}", db_path);
    
    let proxy_url = std::env::var("KEY_PROXY_URL").unwrap_or_else(|_| "http://127.0.0.1:9999".to_string());
    let proxy_provider = Arc::new(infrastructure::llm::proxy::ProxyLlmProvider::new(
        proxy_url,
        "daemon".to_string(),
        "gemini".to_string()
    ));

    // Use the concrete implementation from infrastructure
    let queue = SqliteJobQueue::new(&db_path).await?
        .with_embeddings(proxy_provider);
    
    let queue: Arc<dyn JobQueue> = Arc::new(queue);

    // Initial check
    let count = queue.get_pending_job_count().await?;
    info!("📊 Currently pending jobs: {}", count);

    if count == 0 {
        info!("💡 No jobs found. Enqueuing a sample research task...");
        let req = ConceptRequest {
            topic: "Open Source AI Governance".to_string(),
            category: "research".to_string(),
            trend_items: vec![],
            available_styles: vec!["academic".to_string()],
            relevant_karma: vec![],
            previous_attempt_log: None,
        };
        
        // Match the JobQueue::enqueue signature: (category, topic, style, karma_directives)
        let job_id = queue.enqueue(
            "research", 
            &req.topic, 
            "academic", 
            Some(&serde_json::to_string(&req)?)
        ).await?;
        info!("✅ Enqueued sample job: {}", job_id);
    }

    info!("🛡️  Daemon is now standing by.");
    
    // Simulate some work
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    info!("🏁 aiome-daemon demo run complete.");

    Ok(())
}
