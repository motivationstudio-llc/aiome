use shared::config::FactoryConfig;
use shared::security::SecurityPolicy;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::trend_sonar::BraveTrendSonar;
use infrastructure::media_forge::MediaForgeClient;
use bastion::fs_guard::Jail;
use std::sync::Arc;
use std::time::Duration;

mod supervisor;
mod orchestrator;
mod arbiter;
mod asset_manager;
mod server;
mod simulator;
mod job_worker;
use job_worker::JobWorker;
use server::telemetry::TelemetryHub;
use server::router::{create_router, AppState};
use supervisor::{Supervisor, SupervisorPolicy};
use orchestrator::ProductionOrchestrator;
use arbiter::ResourceArbiter;
use factory_core::traits::{AgentAct, JobQueue};
use infrastructure::concept_manager::ConceptManager;
use infrastructure::voice_actor::VoiceActor;
use infrastructure::sound_mixer::SoundMixer;
use shared::health::HealthMonitor;
use tokio::signal;
use tracing::{info, error, warn};
use tokio::sync::Mutex;
use sidecar::SidecarManager;
use std::process::Command;

use clap::Parser;
use tuning::StyleManager;
use asset_manager::AssetManager;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// 通常の動画生成モード
    Generate {
        /// 動画のカテゴリ
        #[arg(short, long, default_value = "tech")]
        category: String,

        /// 動画のトピック (テーマ)
        #[arg(short, long, default_value = "AIの未来")]
        topic: String,

        /// Remix 対象の動画ID (workspace/<ID> を再利用)
        #[arg(short, long)]
        remix: Option<String>,

        /// スキップ先のステップ (voice, visual)
        #[arg(short, long)]
        step: Option<String>,
    },
    /// 指令センター用サーバーモード (Port: 3000)
    Serve {
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// SNS動画IDをジョブに紐付ける (The Anchor Link)
    LinkSns {
        /// 紐付け対象のジョブID
        #[arg(short, long)]
        job_id: String,
        /// 投稿プラットフォーム (youtube, tiktok, etc.)
        #[arg(short, long, default_value = "youtube")]
        platform: String,
        /// SNS側の動画ID
        #[arg(short, long)]
        video_id: String,
    },
    /// 進化の妥当性検証シミュレーター (Phase 11 Step 4)
    SimulateEvolution,
    /// 今すぐ Samsara プロトコル（合成・エンキュー）を実行する
    SamsaraNow,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    // 0.1. Watchtower Logging & Heartbeat (The Backpressure Trap Fix)
    // ログ転送用のチャネルを作成 (容量1000)
    use shared::watchtower::CoreEvent;
    let (log_tx, log_rx) = tokio::sync::mpsc::channel::<CoreEvent>(1000);
    let log_layer = server::watchtower::LogDrain::new(log_tx.clone());

    // Job Channel for Watchtower Commands
    use factory_core::contracts::WorkflowRequest;
    let (job_tx, mut job_rx) = tokio::sync::mpsc::channel::<WorkflowRequest>(100);
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(log_layer)
        .init();

    let args = Args::parse();

    // 0.2. Watchtower UDS Server — deferred to after job_queue init (line ~190)
    //       log_rx and job_tx are passed later.

    // Status tracking for Heartbeat
    let current_job = Arc::new(Mutex::new(Option::<String>::None));

    // 0.3. Heartbeat Loop
    {
        let tx = log_tx.clone();
        let health = Arc::new(Mutex::new(HealthMonitor::new()));
        let current_job = current_job.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                let status = health.lock().await.check();
                let job_id = current_job.lock().await.clone();
                let sys_status = shared::watchtower::SystemStatus {
                    cpu_usage: status.cpu_usage_percent,
                    memory_used_mb: status.memory_usage_mb,
                    vram_used_mb: 0, 
                    active_job_id: job_id, 
                };
                if let Err(_) = tx.try_send(shared::watchtower::CoreEvent::Heartbeat(sys_status)) {
                    // Drop
                }
            }
        });
    }

    // 0. 初期化: PGID設定
    // 自身をプロセスグループリーダーに昇格させることで、kill -PGID で確実に子プロセスまで殲滅可能にする
    nix::unistd::setpgid(nix::unistd::Pid::from_raw(0), nix::unistd::Pid::from_raw(0)).ok();
    
    // PIDファイルの作成 (The ID Card)
    let pid = std::process::id();
    std::fs::write("/tmp/aiome.id", pid.to_string())?;
    tracing::info!("🆔 Process Group Leader Established. PID: {}", pid);

    // 0.5. 運用監視 (Phase 3)
    let health = Arc::new(Mutex::new(HealthMonitor::new()));
    let status = health.lock().await.check();
    tracing::info!("📊 Initial Health Status: Memory {}MB, CPU {:.1}%", 
        status.memory_usage_mb, status.cpu_usage_percent);

    // 1. 設定を読み込む
    let config = FactoryConfig::default();
    let policy = SecurityPolicy::default_production();

    tracing::info!("⚙️  Config loaded:");
    tracing::info!("   Ollama:   {}", config.ollama_url);
    tracing::info!("   ComfyUI:  {}", config.comfyui_api_url);
    tracing::info!("   Model:    {}", config.model_name);

    // 2. セキュリティレイヤー (Bastion) の初期化
    tracing::info!("🔒 Industrial Security Layer (BASTION) Initializing...");
    let shield = Arc::new(policy.shield().clone());
    
    // 物理的リスク対策: 檻 (Jail) の位置をプロジェクト内の workspace に強制同期
    let jail_path = std::env::current_dir()?.join("workspace/shorts_factory");
    let jail = Arc::new(Jail::init(&jail_path)?);
    
    // ComfyUI 出力先の物理的同期用ディレクトリ作成
    let comfy_out = jail_path.join(&config.comfyui_base_dir);
    if !comfy_out.exists() {
        std::fs::create_dir_all(&comfy_out)?;
    }

    // DX向上対策: Guardrail Enforcement 状態の表示
    let enforce = std::env::var("ENFORCE_GUARDRAIL")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);
    tracing::info!("🛡️  Guardrails Enforcement: {}", if enforce { "Strict (DENY)" } else { "Relaxed (WARN)" });
    tracing::info!("📂 Jail Root: {}", jail_path.display());
    tracing::info!("📁 ComfyUI Sync: {}", comfy_out.display());
    
    // 3. 統治機構 (Supervisor) の初期化
    let supervisor = Supervisor::new(jail.clone(), SupervisorPolicy::Retry { max_retries: 3 });
    tracing::info!("⚖️  Governance Layer (Lex AI) Active");

    // 4. 新規マネージャの初期化 (Phase 8)
    let style_path = std::env::current_dir()?.join("styles.toml");
    let style_manager = Arc::new(StyleManager::load_from_file(style_path).unwrap_or_else(|_| {
        warn!("⚠️ styles.toml not found, using empty manager");
        StyleManager::new_empty()
    }));
    
    let asset_manager = Arc::new(AssetManager::new(std::env::current_dir()?.join("workspace")));

    // 5. インフラクライアントの準備
    let arbiter = Arc::new(ResourceArbiter::new());

    // 5.1 The Persistent Memory & The Samsara Protocol
    let db_dir = std::env::current_dir()?.join("workspace").join("db");
    if !db_dir.exists() {
        std::fs::create_dir_all(&db_dir)?;
    }
    let db_filepath = format!("sqlite://{}", db_dir.join("shorts_factory.db").display());
    let job_queue = Arc::new(infrastructure::job_queue::SqliteJobQueue::new(&db_filepath).await?.with_embeddings(&config.gemini_api_key));

    // 5.2 Dual-Core Soul Architecture (SOUL.md = Master + EVOLVING_SOUL.md = Mutable)
    let cwd = std::env::current_dir()?;
    let soul_md_path = cwd.join("SOUL.md");
    let evolving_soul_path = cwd.join("EVOLVING_SOUL.md");
    let master_soul = std::fs::read_to_string(&soul_md_path).unwrap_or_else(|_| {
        warn!("⚠️ SOUL.md not found at {}. Using default soul.", soul_md_path.display());
        "## Default Soul\n- Be creative.\n- Stay true to the mission.".to_string()
    });
    let evolving_soul = std::fs::read_to_string(&evolving_soul_path).unwrap_or_else(|_| {
        info!("ℹ️ EVOLVING_SOUL.md not found. Starting with blank evolving soul.");
        String::new()
    });
    let soul_md = format!("{}\n\n---\n# Evolving Soul (自律進化領域)\n{}", master_soul, evolving_soul);

    // 5.3 WASM Self-Evolution Infrastructure (SkillForge & WasmSkillManager)
    let skills_dir = cwd.join("workspace/skills");
    let forge_template = cwd.join("workspace/skill_generator");
    let skill_manager = Arc::new(infrastructure::skills::WasmSkillManager::new(&skills_dir, &cwd)
        .map_err(|e| anyhow::anyhow!(e))?
        .with_limits(100 * 1024 * 1024, Duration::from_secs(10)));
    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(&forge_template, &skills_dir));
    let forge_prompt_path = cwd.join("workspace/config/SKILL_FORGE_PROMPT.md");
    let skill_forge_prompt = std::fs::read_to_string(&forge_prompt_path).unwrap_or_else(|_| {
        warn!("⚠️ SKILL_FORGE_PROMPT.md not found at {}. Using empty prompt.", forge_prompt_path.display());
        String::new()
    });

    // 0.2. Start Watchtower UDS Server (deferred — needs job_queue Arc)
    let wt_server = server::watchtower::WatchtowerServer::new(
        log_rx, 
        log_tx.clone(), 
        job_tx, 
        job_queue.clone(),
        config.gemini_api_key.clone(),
        soul_md.clone(),
        config.ollama_url.clone(),
        "huihui_ai/mistral-small-abliterated:latest".to_string(), // 規制解除版 Mistral-Small
        config.unleashed_mode,
        skill_manager,
        skill_forge,
        skill_forge_prompt,
    );
    tokio::spawn(wt_server.start());

    let _cron_scheduler = server::cron::start_cron_scheduler(
        job_queue.clone(),
        log_tx.clone(),
        config.ollama_url.clone(),
        config.model_name.clone(),
        config.brave_api_key.clone(),
        config.youtube_api_key.clone(),
        config.gemini_api_key.clone(),
        soul_md.clone(),
        config.workspace_dir.clone(),
        config.comfyui_base_dir.clone(),
        config.clean_after_hours,
    ).await.map_err(|e| factory_core::error::FactoryError::Infrastructure { reason: format!("Cron failed to start: {}", e) })?;
    info!("🌙 Samsara Protocol is now ACTIVE (Proactive Watchtower enabled)");

    // Sidecar Manager ("The Reaper")
    let sidecar_manager = Arc::new(SidecarManager::new(vec![
        "python".to_string(), "python3".to_string(), "Python".to_string(), "uv".to_string(), "main".to_string(), "shorts-factory".to_string(), "shorts-fa".to_string()
    ]));

    let should_spawn_tts = match &args.command {
        Some(Commands::Serve { .. }) | Some(Commands::Generate { .. }) | None => true,
        _ => false,
    };

    // TTS Sidecar (Qwen3-TTS)
    if should_spawn_tts {
        let sm = sidecar_manager.clone();
        sm.clean_port(5001).await?;
        // TIME_WAIT ソケット解放を待機
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mut cmd = Command::new(".venv/bin/python");
        cmd.arg("tts_server.py")
           .env("PYTORCH_ENABLE_MPS_FALLBACK", "1")
           .current_dir("services/qwen3-tts");
        sm.spawn(cmd).await?;
        info!("🎙️  TTS Sidecar server (Qwen3-TTS) spawned on port 5001");
        // コールドスタート（モデルロード）待機
        tokio::time::sleep(Duration::from_secs(10)).await;
    }


    // Infrastructure Clients
    let trend_sonar = BraveTrendSonar::new(config.brave_api_key.clone());
    let concept_manager = ConceptManager::new(&config.gemini_api_key, &config.script_model);
    let comfy_bridge = ComfyBridgeClient::new(
        shield.clone(),
        &config.comfyui_api_url,
        &config.comfyui_base_dir,
        config.comfyui_timeout_secs,
    );
    let voice_actor = VoiceActor::new("http://localhost:5001", "aiome_narrator");
    let bgm_path = std::env::current_dir()?.join("resources/bgm");
    if !bgm_path.exists() {
        std::fs::create_dir_all(&bgm_path)?;
    }
    let sound_mixer = SoundMixer::new(bgm_path);
    let media_forge = MediaForgeClient::new(jail.clone());

    // 6. 生産ライン・オーケストレーターの準備
    let orchestrator = Arc::new(ProductionOrchestrator::new(
        trend_sonar,
        concept_manager,
        voice_actor,
        comfy_bridge,
        media_forge,
        sound_mixer,
        supervisor,
        arbiter,
        style_manager.clone(),
        asset_manager.clone(),
        config.export_dir.clone(),
    ));

    // コマンド分岐
    match args.command.unwrap_or(Commands::Generate { 
        category: "tech".to_string(), 
        topic: "AIの未来".to_string(), 
        remix: None, 
        step: None 
    }) {
        Commands::Serve { port } => {
            info!("📡 Starting Command Center Server on port {}", port);
            
            // Telemetry Hub
            let telemetry = Arc::new(TelemetryHub::new());
            telemetry.start_heartbeat_loop().await;

            // 6.2 Autonomous JobWorker (The Autonomous Engine)
            let _worker = Arc::new(JobWorker::new(
                job_queue.clone(),
                orchestrator.clone(),
                jail.clone(),
                soul_md.clone(),
            ));
            // tokio::spawn(worker.start_loop());

            // Axum Router
            let state = Arc::new(AppState {
                telemetry,
                orchestrator,
                style_manager,
                jail,
                is_busy: Arc::new(std::sync::Mutex::new(false)),
                asset_manager,
                current_job: current_job.clone(),
                job_queue: job_queue.clone(),
            });
            let worker_state = state.clone(); 
            tokio::spawn(async move {
                while let Some(req) = job_rx.recv().await {
                   info!("🏗️ Processing Watchtower Job: {}", req.topic);
                   
                   // 1. Try acquire lock
                   let acquired = {
                       if let Ok(mut busy) = worker_state.is_busy.try_lock() {
                           if !*busy {
                               *busy = true;
                               true
                           } else {
                               false
                           }
                       } else {
                           false
                       }
                   };

                   if acquired {
                        // 2. Set current job info
                        {
                            let mut job_info = worker_state.current_job.lock().await;
                            *job_info = Some(format!("{}: {}", req.category, req.topic));
                        }

                        // 3. Execute
                        if let Err(e) = worker_state.orchestrator.execute(req, &worker_state.jail).await {
                            error!("❌ Watchtower Job Failed: {}", e);
                        } else {
                            info!("✅ Watchtower Job Complete");
                        }

                        // 4. Release & Clear job info
                        {
                            let mut job_info = worker_state.current_job.lock().await;
                            *job_info = None;
                        }
                        
                        if let Ok(mut busy) = worker_state.is_busy.lock() {
                            *busy = false;
                            worker_state.telemetry.broadcast_log("INFO", "System Ready (Watchtower Job Done)");
                        }
                    } else {
                        warn!("⚠️ System Busy. Dropping Watchtower Job.");
                    }
                }
            });

            let app = create_router(state);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            axum::serve(listener, app).await?;
        }
        Commands::LinkSns { job_id, platform, video_id } => {
            info!("🔗 Linking Job {} to {} video ID: {}", job_id, platform, video_id);
            match job_queue.link_sns_data(&job_id, &platform, &video_id).await {
                Ok(_) => info!("✅ Linking Successful."),
                Err(e) => error!("❌ Failed to link SNS data: {}", e),
            }
        }
        Commands::SimulateEvolution => {
            info!("🔬 Preparing Evolution Simulator environment...");
            if let Err(e) = simulator::run_evolution_simulation(
                job_queue.pool_ref(),
                &config.gemini_api_key,
                "gemini-2.5-flash",
                soul_md.clone(),
            ).await {
                error!("❌ Evolution Simulation Failed: {}", e);
            }
        }
        Commands::SamsaraNow => {
            info!("🔄 [Samsara] Manual trigger initiated. Starting synthesis...");
            let config = FactoryConfig::default();
            match server::cron::synthesize_next_job(
                &config.gemini_api_key,
                "gemini-2.5-flash",
                &config.brave_api_key,
                &*job_queue,
            ).await {
                Ok(_) => info!("✅ [Samsara] Manual synthesis complete. Job enqueued."),
                Err(e) => error!("❌ [Samsara] Manual synthesis failed: {}", e),
            }
        }
        Commands::Generate { category, topic, remix, step } => {
            let workflow_req = WorkflowRequest { 
                category: category.clone(), 
                topic: topic.clone(),
                remix_id: remix.clone(),
                skip_to_step: step.clone(),
                style_name: String::new(), 
                custom_style: None,
                target_langs: vec!["ja".to_string(), "en".to_string()],
            };
        
            info!("🚀 Launching Production Pipeline...");
            
            tokio::select! {
                res = orchestrator.execute(workflow_req, &jail) => {
                    match res {
                        Ok(res) => {
                            println!("\n🎬 動画生成完了！");
                            println!("   📝 タイトル: {}", res.concept.title);
                            println!("   🎨 スタイル: {}", res.concept.style_profile);
                            for v in res.output_videos {
                                println!("   🎥 [{}] ファイル: {}", v.lang, v.path);
                            }
                        }
                        Err(e) => {
                            error!("❌ 生成パイプラインが失敗: {}", e);
                        }
                    }
                }
                _ = signal::ctrl_c() => {
                    tracing::info!("🛑 SIGINT received. Shutting down gracefully...");
                }
            }
        }
    }

    Ok(())
}
