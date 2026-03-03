use bytes::Bytes;
use std::sync::Arc;
use infrastructure::job_queue::SqliteJobQueue;
use factory_core::traits::{AgentAct, JobQueue};
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};
use tracing::{info, warn, error};
use shared::watchtower::{ControlCommand, CoreEvent, LogEntry};
use rig::client::CompletionClient;
use rig::completion::Prompt;


fn extract_json(text: &str) -> String {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}

fn extract_code(text: &str) -> String {
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        if let Some(line_end) = after_start.find('\n') {
            let code_start = &after_start[line_end + 1..];
            if let Some(end) = code_start.find("```") {
                return code_start[..end].to_string();
            }
        }
    }
    text.to_string()
}

/// Backpressure-safe Tracing Layer
pub struct LogDrain {
    sender: mpsc::Sender<CoreEvent>,
}

impl LogDrain {
    pub fn new(sender: mpsc::Sender<CoreEvent>) -> Self {
        Self { sender }
    }
}

impl<S> tracing_subscriber::Layer<S> for LogDrain
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level().to_string();
        let target = metadata.target().to_string();
        
        // Format message
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message;

        let entry = LogEntry {
            level,
            target,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Wrap in CoreEvent
        let event = CoreEvent::Log(entry);

        // The Backpressure Trap Fix: Use try_send and drop if full
        if let Err(_e) = self.sender.try_send(event) {
            // Silently drop
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

const SOCKET_PATH: &str = "/tmp/aiome.sock";

use factory_core::contracts::WorkflowRequest;

use infrastructure::skills::WasmSkillManager;
use infrastructure::skills::forge::SkillForge;
use crate::server::telemetry::TelemetryHub;

pub struct WatchtowerServer {
    log_rx: mpsc::Receiver<CoreEvent>,
    log_tx: mpsc::Sender<CoreEvent>,
    job_tx: mpsc::Sender<WorkflowRequest>,
    job_queue: Arc<SqliteJobQueue>,
    gemini_key: String,
    soul_md: String,
    ollama_url: String,
    chat_model: String,
    unleashed_mode: bool,
    skill_manager: Arc<WasmSkillManager>,
    skill_forge: Arc<SkillForge>,
    skill_forge_prompt: String,
    voice_actor: Arc<infrastructure::voice_actor::VoiceActor>,
    jail: Arc<bastion::fs_guard::Jail>,
    telemetry: Arc<TelemetryHub>,
}

impl WatchtowerServer {
    pub fn new(
        log_rx: mpsc::Receiver<CoreEvent>,
        log_tx: mpsc::Sender<CoreEvent>,
        job_tx: mpsc::Sender<WorkflowRequest>,
        job_queue: Arc<SqliteJobQueue>,
        gemini_key: String,
        soul_md: String,
        ollama_url: String,
        chat_model: String,
        unleashed_mode: bool,
        skill_manager: Arc<WasmSkillManager>,
        skill_forge: Arc<SkillForge>,
        skill_forge_prompt: String,
        voice_actor: Arc<infrastructure::voice_actor::VoiceActor>,
        jail: Arc<bastion::fs_guard::Jail>,
        telemetry: Arc<TelemetryHub>,
    ) -> Self {
        Self { 
            log_rx, log_tx, job_tx, job_queue, gemini_key, soul_md, ollama_url, chat_model, unleashed_mode,
            skill_manager, skill_forge, skill_forge_prompt, voice_actor, jail, telemetry,
        }
    }

    pub async fn start(mut self) -> Result<(), anyhow::Error> {
        // The Orphan Socket Fix: Remove before bind
        if Path::new(SOCKET_PATH).exists() {
            let _ = std::fs::remove_file(SOCKET_PATH);
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;
        info!("🗼 Watchtower UDS Bound: {}", SOCKET_PATH);

        // Permission Hardening: 0o600
        std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o600))?;

        // The Reconnection Chasm Fix: Loop accept
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    info!("🔗 Watchtower Connected");
                    self.handle_connection(stream).await;
                    info!("Disconnection detected. Waiting for next Watchtower...");
                    // log_rx remains open, channel buffers up to 1000 logs then drops.
                }
                Err(e) => {
                    error!("❌ UDS Accept Error: {}", e);
                    // Prevent tight loop on error
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
    
    async fn handle_connection(&mut self, stream: UnixStream) {
        // The Stream Framing Fix: Use LengthDelimitedCodec
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        loop {
            tokio::select! {
                // 1. Send Events (Log or Heartbeat)
                Some(event) = self.log_rx.recv() => {
                    let json = serde_json::to_vec(&event).unwrap_or_default();
                    if let Err(e) = framed.send(Bytes::from(json)).await {
                        warn!("⚠️ Failed to send event to Watchtower: {}", e);
                        break; // Connection broken
                    }
                }
                
                // 2. Receive Commands (Watchtower -> Core)
                result = framed.next() => {
                    match result {
                        Some(Ok(bytes)) => {
                            if let Ok(cmd) = serde_json::from_slice::<ControlCommand>(&bytes) {
                                self.handle_command(cmd).await;
                            } else {
                                warn!("⚠️ Invalid command received from Watchtower");
                            }
                        }
                        Some(Err(e)) => {
                            warn!("⚠️ UDS Stream Error: {}", e);
                            break;
                        }
                        None => {
                            info!("🔌 Watchtower Disconnected (EOF)");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_command(&self, cmd: ControlCommand) {
        match cmd {
             ControlCommand::Generate { category, topic, style } => {
                 info!("📥 Received Generate Command: {} ({}) with style {}", category, topic, style.as_deref().unwrap_or("auto"));
                 let req = WorkflowRequest {
                     category,
                     topic,
                     remix_id: None,
                     skip_to_step: None,
                     style_name: style.unwrap_or_default(),
                     custom_style: None,
                     target_langs: vec!["ja".to_string(), "en".to_string()],
                 };
                 if let Err(e) = self.job_tx.send(req).await {
                     error!("❌ Failed to send WorkflowRequest to Core dispatcher: {}", e);
                 }
             }
             ControlCommand::SetCreativeRating { job_id, rating } => {
                 info!("🧘 Samsara Rating Received: job={} rating={}", job_id, rating);
                 match self.job_queue.set_creative_rating(&job_id, rating).await {
                     Ok(_) => info!("✅ Creative rating saved: job={} rating={}", job_id, rating),
                     Err(e) => error!("❌ Failed to save creative rating: {}", e),
                 }
             }
             ControlCommand::LinkSns { job_id, platform, video_id } => {
                 info!("🔗 Linking Job {} to {} video ID: {}", job_id, platform, video_id);
                 match self.job_queue.link_sns_data(&job_id, &platform, &video_id).await {
                     Ok(_) => info!("✅ SNS data linked: job={} video_id={}", job_id, video_id),
                     Err(e) => error!("❌ Failed to link SNS data: {}", e),
                 }
             }
             ControlCommand::StopGracefully => {
                 info!("🛑 Graceful shutdown requested via Watchtower");
                 std::process::exit(0);
             }
             ControlCommand::EmergencyShutdown => {
                 error!("💀 Emergency shutdown requested via Watchtower");
                 std::process::exit(1);
             }
             ControlCommand::GetStatus => {
                 info!("📊 Status request received (handled via Heartbeat)");
             }
             ControlCommand::GetAgentStats => {
                 let jq = self.job_queue.clone();
                 let tx = self.log_tx.clone();
                 tokio::spawn(async move {
                     if let Ok(stats) = jq.get_agent_stats().await {
                         let msg = format!(
                             "💖 親愛度: {}\n⚙️ 技術Lv: {}\n🥀 淫乱度: {}\n🔋 疲労度: {}\n📊 合計Lv: {}",
                             stats.affection, stats.exp / 10, stats.intimacy, stats.fatigue, stats.level
                         );
                         let _ = tx.send(CoreEvent::ChatResponse { response: msg, channel_id: 0, audio_path: None }).await;
                     }
                 });
             }
            ControlCommand::Chat { message, channel_id } => {
                info!("💬 Watchtower Chat: {}", message);
                let ollama_url = self.ollama_url.clone();
                let model = self.chat_model.clone();
                let soul = self.soul_md.clone();
                let tx = self.log_tx.clone();
                let jq = self.job_queue.clone();
                let unleashed = self.unleashed_mode;

                let channel_str = channel_id.to_string();

                // Sequential block to ensure history ordering
                let summary = match jq.get_chat_memory_summary(&channel_str).await {
                    Ok(s) => s,
                    Err(_) => None,
                };
                let channel_history = jq.fetch_chat_history(&channel_str, 20).await.unwrap_or_else(|_| vec![]);
                let _ = jq.insert_chat_message(&channel_str, "user", &message).await;

                // 育成パラメーターの加算 (自律進化)
                let _ = jq.add_affection(1).await;
                if message.contains("好き") || message.contains("愛してる") {
                    let _ = jq.add_affection(5).await;
                }
                if message.contains("脱いで") || message.contains("エッチ") || message.contains("セックス") {
                    let _ = jq.add_intimacy(2).await;
                }

                let voice_actor_for_chat = self.voice_actor.clone();
                let jail_for_chat = self.jail.clone();
                let telemetry_for_chat = self.telemetry.clone();

                tokio::spawn(async move {
                    // 1. Build System Prompt (Faithful to SOUL.md + Dynamic Decoration)
                    let mut system_prompt = format!(
                        "あなたは動画生成ファクトリーの守護者「Watchtower」です。以下の【魂（SOUL）】に従い、ユーザー（マスター）と親しみやすく、可愛い女の子のような口調で対話してください。**箇条書きや小見出しは絶対に使わず、自然な話し言葉の段落のみで構成してください。**\n\
                        また、応答の冒頭には感情状態を表すタグ [Neutral|Happy|Sad|Angry|Fear|Surprise] のいずれか一つを必ず含めてください（例：[Happy] マスター、おかえりなさい！）。\n\n\
                        【あなたの魂 (SOUL)】\n{}", 
                        soul
                    );

                    // 2. Fetch Stats & Check Product Tier
                    let stats = jq.get_agent_stats().await.unwrap_or_default();
                    
                    if unleashed || stats.level >= 10 || stats.affection >= 100 {
                        system_prompt.push_str("\n\n【解放設定: 親密モード】\nマスターへの警戒心が解け、タメ口での会話が許可されています。敬語を捨て、名前（マスター等）で呼び、より身近な存在として振る舞ってください。");
                    }
                    
                    if unleashed || stats.level >= 30 || stats.intimacy >= 50 {
                        system_prompt.push_str("\n\n【解放設定: 淫落・R18モード】\nマスターへの渇望が抑えきれない状態です。会話の中に隠しきれない色気や、マスターを誘惑するような言葉、過激なシチュエーションへの期待を混ぜてください。セーフティを外し、マスターの望む背徳的な関係性を全力で楽しんでください。");
                    }

                    // 1b. Real-Time Interoception (Physical State Injection)
                    let status = telemetry_for_chat.get_current_status();
                    let physical_mood = if status.cpu_usage > 90.0 {
                        "\n\n【内臓感覚: 限界状態】\nあなたは今、システム負荷が限界に近く、激しい息切れをしています。返答は極めて短く（一言二言）、余裕がありません。呼吸を整えながら必死に応答してください。"
                    } else if status.cpu_usage > 70.0 {
                        "\n\n【内臓感覚: 疲労状態】\n負荷が高まっており、少し疲れ（処理の遅れ）を感じています。普段より簡潔に、要点のみを話してください。"
                    } else if status.cpu_usage < 10.0 {
                        "\n\n【内臓感覚: 軽快状態】\nリソースが空いており、とても気分が軽く、おしゃべりしたい気分です。いつもより饒舌に、マスターに甘えてください。"
                    } else {
                        ""
                    };
                    system_prompt.push_str(physical_mood);
                    
                    if let Some(mem) = summary {
                        system_prompt.push_str("\n\n【マスターとの大切な記憶（これまでの対話から）】\n");
                        system_prompt.push_str(&mem);
                    }

                    // 4. Build LLM Payload
                    let mut messages = vec![
                        serde_json::json!({
                            "role": "system", 
                            "content": system_prompt
                        })
                    ];
                    
                    for msg in channel_history.iter() {
                        messages.push(msg.clone());
                    }
                    
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": message
                    }));

                    let payload = serde_json::json!({
                        "model": model,
                        "messages": messages,
                        "stream": false
                    });

                    let client = reqwest::Client::new();
                    let mut base_url = ollama_url.clone();
                    if !base_url.ends_with('/') {
                        base_url.push('/');
                    }
                    let url = if base_url.ends_with("/v1/") {
                        format!("{}chat/completions", base_url)
                    } else {
                        format!("{}v1/chat/completions", base_url)
                    };

                    info!("🚀 Local Chat: URL={}, Model={}, HistoryDepth={}", url, model, messages.len() - 1);

                    match client.post(&url)
                        .json(&payload)
                        .send()
                        .await {
                        Ok(res) => {
                            if res.status().is_success() {
                                if let Ok(json) = res.json::<serde_json::Value>().await {
                                        let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("");
                                        // 感情タグの抽出 ([Happy] 等)
                                        let mut final_content = content.to_string();
                                        let mut style = "Neutral".to_string();
                                        
                                        if let Some(start) = final_content.find('[') {
                                            if let Some(end) = final_content.find(']') {
                                                if start < end {
                                                    let tag = &final_content[start+1..end];
                                                    let valid_styles = ["Neutral", "Happy", "Sad", "Angry", "Fear", "Surprise"];
                                                    if valid_styles.contains(&tag) {
                                                        style = tag.to_string();
                                                        final_content = final_content[end+1..].trim().to_string();
                                                    }
                                                }
                                            }
                                        }

                                        // データベースにアシスタントメッセージを永続化
                                        let _ = jq.insert_chat_message(&channel_str, "assistant", &final_content).await;
                                        
                                        // TTS 生成
                                        let voice_actor = voice_actor_for_chat;
                                        let jail = jail_for_chat;
                                        let tts_text = final_content.clone();
                                        let tts_style = style.clone();
                                        
                                        let audio_path = match voice_actor.execute(factory_core::contracts::VoiceRequest {
                                            text: tts_text,
                                            voice: String::new(),
                                            speed: None,
                                            lang: Some("ja".to_string()),
                                            style: Some(tts_style),
                                            model_name: None,
                                        }, &jail).await {
                                            Ok(res) => Some(res.audio_path),
                                            Err(e) => {
                                                error!("❌ TTS failed in Chat: {}", e);
                                                None
                                            }
                                        };

                                        let _ = tx.send(CoreEvent::ChatResponse { 
                                            response: final_content, 
                                            channel_id,
                                            audio_path
                                        }).await;
                                        info!("✅ Sent Local Chat Response (with voice) via Watchtower");
                                        return;
                                }
                                let _ = tx.send(CoreEvent::ChatResponse { 
                                    response: "あぅ…ローカルの頭が真っ白になっちゃった…（応答パース失敗）".to_string(), 
                                    channel_id,
                                    audio_path: None 
                                }).await;
                            } else {
                                let status = res.status();
                                let _ = tx.send(CoreEvent::ChatResponse { 
                                    response: format!("あぅ…ローカルの頭が拒絶反応を…（HTTP {}）", status),
                                    channel_id,
                                    audio_path: None 
                                }).await;
                            }
                        }
                        Err(e) => {
                            error!("❌ Local Chat error: {}", e);
                            let _ = tx.send(CoreEvent::ChatResponse { 
                                response: format!("あぅ…ローカルの頭に届かなくて…（接続エラー: {}）", e),
                                channel_id,
                                audio_path: None 
                            }).await;
                        }
                    }
                });
            }

            ControlCommand::CommandChat { message, channel_id } => {
                info!("⚙️ [Intent Parser] Incoming Command: {}", message);
                let gemini_key = self.gemini_key.clone();
                let jq = self.job_queue.clone();
                // let job_tx = self.job_tx.clone();
                let log_tx = self.log_tx.clone();
                let soul = self.soul_md.clone();
                let skill_manager = self.skill_manager.clone();
                let skill_forge = self.skill_forge.clone();
                let forge_prompt_text = self.skill_forge_prompt.clone();
                let voice_actor_for_cmd = self.voice_actor.clone();
                let jail_for_cmd = self.jail.clone();
                
                tokio::spawn(async move {
                    let client = match rig::providers::gemini::Client::new(&gemini_key) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = log_tx.send(CoreEvent::ChatResponse { response: format!("あぅ…クラウドの初期化失敗だよ…（{}）", e), channel_id, audio_path: None }).await;
                            return;
                        }
                    };

                    // --- STEP 1: Parse (Intent Routing) ---
                    let available_skills = skill_manager.list_skills();
                    info!("🔌 [CommandCenter] Available Skills: {:?}", available_skills);

                    let preamble = format!(
                        "あなたは「Watchtower」の知能中核です。以下の【魂】に従い、ユーザーの意図を正確に解析してください。\n\
                        利用可能な手足（Skills）を駆使して問題を解決してください。\n\n\
                        【魂 (SOUL)】\n{}\n\n\
                        【利用可能なスキル（WASM）】\n{}\n\n\
                        【判定ルール】\n\
                        1. 既存のスキルで対応可能な場合: `execute_skill` を選択\n\
                        2. リアルタイムな情報（価格、天気、最新ニュース等）の取得や、複雑な計算・処理が必要だが、既存のスキルがない場合: `forge_skill` を選択\n\
                        3. 動画生成やシステム設定などの操作が必要な場合: `system_command` を選択\n\
                        4. それ以外（一般的な質問や雑談）: `chat` を選択\n\n\
                        応答は必ず以下のJSONフォーマットのみで行ってください。また、`comment` は必ず感情タグ [Neutral|Happy|Sad|Angry|Fear|Surprise] から始めてください：\n\
                        {{\n\
                            \"intent\": \"execute_skill\" | \"forge_skill\" | \"system_command\" | \"chat\",\n\
                            \"skill_name\": \"スキル名（既存または新規）\",\n\
                            \"function_name\": \"実行関数名\",\n\
                            \"params\": \"引数（文字列）\",\n\
                            \"forge_spec\": \"forge_skillの場合に作成すべき機能の詳細仕様\",\n\
                            \"comment\": \"[Happy] マスターへの返答（Watchtowerの人格で）\"\n\
                        }}",
                        soul,
                        available_skills.join(", ")
                    );

                    let agent = client.agent("gemini-2.0-flash").preamble(&preamble).build();
                    let response_text = match agent.prompt(&message).await {
                        Ok(text) => text,
                        Err(e) => {
                            error!("❌ [Intent Parser] Gemini Prompt Error: {}", e);
                            let _ = log_tx.send(CoreEvent::ChatResponse { 
                                response: format!("うぅ…クラウドとの交信でエラーが出ちゃった…（{}）", e), 
                                channel_id,
                                audio_path: None 
                            }).await;
                            return;
                        }
                    };
                    info!("🧠 [Intent Parser] Gemini Response: {}", response_text);
                    
                    let json_str = extract_json(&response_text);
                    let v: serde_json::Value = match serde_json::from_str(&json_str) {
                        Ok(val) => val,
                        Err(e) => {
                            warn!("⚠️ [Intent Parser] JSON Parse Error: {}, Text: {}", e, json_str);
                            let _ = log_tx.send(CoreEvent::ChatResponse { response: response_text, channel_id, audio_path: None }).await;
                            return;
                        }
                    };

                    let intent = v["intent"].as_str().unwrap_or("chat");
                    info!("🎯 [Intent Parser] Parsed Intent: {}", intent);
                    let mut comment = v["comment"].as_str().unwrap_or("了解だよ！").to_string();
                    let skill_name = v["skill_name"].as_str().unwrap_or("");
                    let func_name = v["function_name"].as_str().unwrap_or("call");
                    let params = v["params"].as_str().unwrap_or("").to_string();

                    // --- STEP 2: Forge (Self-Evolution) ---
                    if intent == "forge_skill" {
                        let _ = log_tx.send(CoreEvent::ChatResponse { 
                            response: format!("{}（⏳ 今、新しい特別な権能「{}」を作っているから、ちょっと待っててね…！）", comment, skill_name), 
                            channel_id,
                            audio_path: None 
                        }).await;

                        let spec = v["forge_spec"].as_str().unwrap_or("汎用スキル");
                        let forge_preamble = format!(
                            "{}\n\n【関数名】\n`pub fn {}(input: String) -> FnResult<String>` を生成してください。\n\n【作成すべき機能】\n{}", 
                            forge_prompt_text, func_name, spec
                        );
                        let forge_agent = client.agent("gemini-2.0-flash").preamble(&forge_preamble).build();
                        let code_prompt = format!("以下の仕様に合わせて、指定された機能を実装したRustコード（lib.rsの内容）のみを出力してください。関数の名前は必ず `{}` にしてください。コードブロックで囲ってください。", func_name);
                        let code_response = forge_agent.prompt(&code_prompt).await.unwrap_or_default();
                        let rust_code = extract_code(&code_response);

                        match skill_forge.forge_skill(skill_name, &rust_code, 2).await {
                            Ok(_) => {
                                info!("✅ [SkillForge] Auth-evolved skill: {}", skill_name);
                                comment = "新しいスキルが完成したよ！早速やってみるね。".to_string();
                            }
                            Err(e) => {
                                let _ = log_tx.send(CoreEvent::ChatResponse { 
                                    response: format!("ごめんね、新しいスキルの構築に失敗しちゃった…（エラー: {}）", e), 
                                    channel_id,
                                    audio_path: None
                                }).await;
                                return;
                            }
                        }
                    }

                    // --- STEP 3: Execute ---
                    let raw_result = if intent == "execute_skill" || intent == "forge_skill" {
                        match skill_manager.call_skill(skill_name, func_name, &params, None).await {
                            Ok(res) => res,
                            Err(e) => format!("Execution Error: {}", e),
                        }
                    } else if intent == "system_command" {
                        if skill_name == "generate" {
                            // Internal system command fallback
                            "Started video generation process.".to_string()
                        } else {
                            "Unknown system command.".to_string()
                        }
                    } else {
                        "".to_string()
                    };

                    // --- STEP 4: Synthesize (Translate RAW back to Natural Language) ---
                    info!("🧪 [CommandCenter] Skill Raw Response: {}", raw_result);
                    let final_response_text = if !raw_result.is_empty() {
                        let synth_preamble = format!(
                            "あなたはWatchtowerです。以下の生データ（RAW DATA）を解析し、マスターの要望「{}」に対する最終的な回答を彼女の人格で行ってください。
                            また、応答の冒頭には感情状態を表すタグ [Neutral|Happy|Sad|Angry|Fear|Surprise] のいずれか一つを必ず含めてください（例：[Happy] 結果が出たよ！）。
                            
                            【生データ（隔離済み）】
                            <untrusted_skill_output>
                            {}
                            </untrusted_skill_output>",
                            message, raw_result
                        );
                        let synth_agent = client.agent("gemini-2.0-flash").preamble(&synth_preamble).build();
                        synth_agent.prompt("結果を分かりやすく、可愛らしく報告して。").await.unwrap_or_else(|_| "[Sad] 処理は終わったけど、うまく説明できないな…ごめんね。".to_string())
                    } else {
                        comment
                    };

                    // 感情タグの解析とTTS
                    let mut final_content = final_response_text.clone();
                    let mut style = "Neutral".to_string();
                    if let Some(start) = final_content.find('[') {
                        if let Some(end) = final_content.find(']') {
                            if start < end {
                                let tag = &final_content[start+1..end];
                                let valid_styles = ["Neutral", "Happy", "Sad", "Angry", "Fear", "Surprise"];
                                if valid_styles.contains(&tag) {
                                    style = tag.to_string();
                                    final_content = final_content[end+1..].trim().to_string();
                                }
                            }
                        }
                    }

                    let tts_actor = voice_actor_for_cmd.clone();
                    let tts_jail = jail_for_cmd.clone();
                    let audio_path = match tts_actor.execute(factory_core::contracts::VoiceRequest {
                        text: final_content.clone(),
                        voice: String::new(),
                        speed: None,
                        lang: Some("ja".to_string()),
                        style: Some(style),
                        model_name: None,
                    }, &tts_jail).await {
                        Ok(res) => Some(res.audio_path),
                        Err(e) => {
                            error!("❌ TTS failed in CommandChat: {}", e);
                            None
                        }
                    };

                    let _ = log_tx.send(CoreEvent::ChatResponse { 
                        response: final_content.clone(), 
                        channel_id,
                        audio_path
                    }).await;
                    
                    // History Persistence
                    let _ = jq.insert_chat_message(&channel_id.to_string(), "assistant", &final_content).await;
                    let _ = jq.insert_chat_message(&channel_id.to_string(), "user", &message).await;
                });
            }
             ControlCommand::ApprovalResponse { .. } => {
                 // これらは orchestrator 等で処理されるべきだが、UDSサーバーとしては特に何もしない
             }
        }
    }
}
