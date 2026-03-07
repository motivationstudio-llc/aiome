/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use poise::serenity_prelude as serenity;
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use shared::watchtower::{ControlCommand, CoreEvent, SystemStatus, LogEntry};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};
use bytes::Bytes;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use anyhow::Context as _; // Import trait for .context() method

use serenity::all::{ChannelId, CreateMessage, CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, ReactionType};

struct Data {
    cmd_tx: mpsc::Sender<ControlCommand>,
    latest_status: Arc<Mutex<Option<SystemStatus>>>,
    log_channel_id: ChannelId,
    command_channel_id: ChannelId,
    chat_channel_id: ChannelId,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type PoiseContext<'a> = poise::Context<'a, Data, Error>;

/// Checking Core status
#[poise::command(slash_command)]
async fn status(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let status_guard = ctx.data().latest_status.lock().await;
    match &*status_guard {
        Some(s) => {
            let msg = format!(
                "🟢 **System Online**
CPU: {:.1}%
RAM: {}MB
VRAM: {}MB
Job: {:?}",
                s.cpu_usage, s.memory_used_mb, s.vram_used_mb, s.active_job_id
            );
            ctx.say(msg).await?;
        }
        None => {
            ctx.say("🔴 **Core Unreachable** (No Heartbeat)").await?;
        }
    }
    Ok(())
}

/// Emergency kill switch (Hybrid Nuke Protocol)
#[poise::command(slash_command, owners_only)]
async fn nuke(
    ctx: PoiseContext<'_>,
    #[description = "Skip graceful shutdown and force kill immediately"] force: Option<bool>,
) -> Result<(), Error> {
    let force = force.unwrap_or(false);

    if !force {
        // Stage 1: Try graceful shutdown via UDS
        ctx.say("⚠️ **Stage 1**: Sending graceful shutdown via UDS...").await?;
        let cmd = ControlCommand::StopGracefully;
        if let Err(_) = ctx.data().cmd_tx.send(cmd).await {
            ctx.say("❌ UDS channel closed. Escalating to Stage 2 (SIGKILL)...").await?;
        } else {
            // Wait 5 seconds for graceful shutdown
            ctx.say("⏳ Waiting 5 seconds for Core to shut down gracefully...").await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            // Check if Core is still alive
            let still_alive = std::fs::read_to_string("/tmp/aiome.id").is_ok();
            if !still_alive {
                ctx.say("✅ **Core shut down gracefully.** No SIGKILL needed.").await?;
                return Ok(());
            }
            ctx.say("⚠️ Core still alive after 5s. Escalating to **Stage 2** (SIGKILL)...").await?;
        }
    } else {
        ctx.say("⚠️ **FORCE MODE**: Skipping graceful shutdown. Going straight to SIGKILL...").await?;
    }

    // Stage 2: SIGKILL via PID file (物理的処刑権限は永久保持)
    match std::fs::read_to_string("/tmp/aiome.id") {
        Ok(pid_str) => {
            let pid: i32 = pid_str.trim().parse()?;
            match signal::kill(Pid::from_raw(-pid), Signal::SIGKILL) {
                Ok(_) => {
                    ctx.say(format!("💀 **Target Destroyed** (PGID: -{}). System halted.", pid)).await?;
                    info!("💀 Executed NUKE Stage 2 (SIGKILL) on PGID -{}", pid);
                }
                Err(e) => {
                    ctx.say(format!("❌ SIGKILL FAILED: {}", e)).await?;
                    error!("Failed to kill PGID -{}: {}", pid, e);
                }
            }
        }
        Err(e) => {
            ctx.say(format!("❌ Cannot read PID file `/tmp/aiome.id`: {}. Core may already be dead.", e)).await?;
        }
    }
    Ok(())
}

/// View Agent Evolution Stats
#[poise::command(slash_command)]
async fn stats(ctx: PoiseContext<'_>) -> Result<(), Error> {
    ctx.data().cmd_tx.send(ControlCommand::GetAgentStats).await?;
    ctx.say("⏳ Fetching emotional and technical stats from Core...").await?;
    Ok(())
}

/// Start a new generation task
#[poise::command(slash_command)]
async fn generate(
    ctx: PoiseContext<'_>,
    #[description = "Category (e.g. tech, nature)"] category: String,
    #[description = "Topic/Theme"] topic: String,
    #[description = "Style Preset"] style: Option<String>,
) -> Result<(), Error> {
    ctx.say(format!("🚀 Dispatching Generate Request: **{}** ({})", topic, category)).await?;
    let cmd = ControlCommand::Generate { category, topic, style };
    if let Err(e) = ctx.data().cmd_tx.send(cmd).await {
        ctx.say(format!("❌ Failed to send command to Core loop: {}", e)).await?;
    } else {
        ctx.say("✅ Request queued for Core.").await?;
    }
    Ok(())
}

/// Talk directly to her (Watchtower/OpenClaw)
#[poise::command(slash_command)]
async fn talk(
    ctx: PoiseContext<'_>,
    #[description = "Message to her"] message: String,
) -> Result<(), Error> {
    let channel_id = ctx.channel_id().get();
    info!("💬 Sending chat command to Core: {}", message);
    let cmd = ControlCommand::Chat { message, channel_id };
    if let Err(e) = ctx.data().cmd_tx.send(cmd).await {
        error!("❌ Failed to send Chat command to Core: {}", e);
        ctx.say(format!("❌ Failed to reach Core: {}", e)).await?;
    } else {
        info!("✅ Chat command sent to Core.");
        ctx.say("💬 ...").await?;
    }
    Ok(())
}

/// Ask her to perform system commands (Command Center)
#[poise::command(slash_command)]
async fn command(
    ctx: PoiseContext<'_>,
    #[description = "Request system action or status"] request: String,
) -> Result<(), Error> {
    let channel_id = ctx.channel_id().get();
    info!("⚙️ Sending CommandChat to Core: {}", request);
    let cmd = ControlCommand::CommandChat { message: request, channel_id };
    if let Err(e) = ctx.data().cmd_tx.send(cmd).await {
        error!("❌ Failed to send CommandChat to Core: {}", e);
        ctx.say(format!("❌ Failed to reach Core: {}", e)).await?;
    } else {
        info!("✅ CommandChat sent to Core.");
        ctx.say("⚙️ ...").await?;
    }
    Ok(())
}

// ... event handler ...


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    
    let token = std::env::var("DISCORD_TOKEN").context("Missing DISCORD_TOKEN")?;
    let log_channel_id: u64 = std::env::var("DISCORD_LOG_CHANNEL_ID")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let command_channel_id: u64 = std::env::var("DISCORD_COMMAND_CHANNEL_ID")
        .unwrap_or_default()
        .parse()
        .unwrap_or(log_channel_id); // Default to log channel if not specified
    let chat_channel_id: u64 = std::env::var("DISCORD_CHAT_CHANNEL_ID")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);

    let latest_status = Arc::new(Mutex::new(None));
    let (event_tx, mut event_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<ControlCommand>(100);

    // === W-1 & W-4: UDS Loop with Reconnection Visibility and Heartbeat Timeout ===
    let status_clone = latest_status.clone();
    let last_heartbeat_time = Arc::new(std::sync::atomic::AtomicI64::new(0));
    let hb_time_writer = last_heartbeat_time.clone();

    // Channel to send Discord messages from the UDS task
    let (discord_tx, mut discord_rx) = mpsc::channel::<String>(50);
    let discord_tx_uds = discord_tx.clone();

    tokio::spawn(async move {
        let mut was_connected = false;
        loop {
            match UnixStream::connect("/tmp/aiome.sock").await {
                Ok(stream) => {
                    if was_connected {
                        let _ = discord_tx_uds.send("🟢 **Core Reconnected.** UDS link restored.".to_string()).await;
                    }
                    was_connected = true;
                    info!("🔗 Connected to Core.");
                    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
                    loop {
                        tokio::select! {
                            // 1. Core -> Bot
                            msg = framed.next() => {
                                match msg {
                                    Some(Ok(bytes)) => {
                                        if let Ok(event) = serde_json::from_slice::<CoreEvent>(&bytes) {
                                            match event {
                                                CoreEvent::Heartbeat(s) => {
                                                    *status_clone.lock().await = Some(s);
                                                    // Update heartbeat timestamp (epoch seconds)
                                                    let now = chrono::Utc::now().timestamp();
                                                    hb_time_writer.store(now, std::sync::atomic::Ordering::Relaxed);
                                                }
                                                _ => { let _ = event_tx.send(event).await; }
                                            }
                                        }
                                    }
                                    _ => break, // Reconnect
                                }
                            }
                            // 2. Bot -> Core
                            Some(cmd) = cmd_rx.recv() => {
                                let json = serde_json::to_vec(&cmd).unwrap_or_default();
                                if let Err(e) = framed.send(Bytes::from(json)).await {
                                    error!("❌ UDS Write Error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    // Connection lost
                    let _ = discord_tx_uds.send("⚠️ **Core Disconnected.** UDS link lost. Retrying in 5s...".to_string()).await;
                    *status_clone.lock().await = None;
                }
                Err(e) => {
                    if !was_connected {
                        // We use warn! instead of error! for initial Retries to reduce noise
                        warn!("⏳ Waiting for Core UDS at /tmp/aiome.sock (is aiome-daemon running?): {}", e);
                    } else {
                        error!("❌ UDS Connection lost at /tmp/aiome.sock: {}", e);
                    }

                    if was_connected {
                        let _ = discord_tx_uds.send("⚠️ **Core Disconnected.** Cannot reach UDS. Retrying in 5s...".to_string()).await;
                        *status_clone.lock().await = None;
                        was_connected = false;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    // === W-4: Heartbeat Sentinel — 30-second timeout watchdog ===
    let hb_time_reader = last_heartbeat_time.clone();
    let discord_tx_sentinel = discord_tx.clone();
    let latest_status_sentinel = latest_status.clone();
    tokio::spawn(async move {
        let mut alerted = false;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let last_hb = hb_time_reader.load(std::sync::atomic::Ordering::Relaxed);
            if last_hb == 0 { continue; } // No heartbeat received yet
            let now = chrono::Utc::now().timestamp();
            let elapsed = now - last_hb;
            if elapsed > 30 && !alerted {
                let _ = discord_tx_sentinel.send(
                    "⚠️ **Heartbeat Lost** — Core may be unresponsive. No heartbeat for 30+ seconds.".to_string()
                ).await;
                *latest_status_sentinel.lock().await = None;
                alerted = true;
            } else if elapsed <= 30 && alerted {
                // Heartbeat recovered
                let _ = discord_tx_sentinel.send(
                    "💚 **Heartbeat Recovered** — Core is responsive again.".to_string()
                ).await;
                alerted = false;
            }
        }
    });

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![status(), nuke(), stats(), generate(), talk(), command()],
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move {
                    // Handle normal messages in specific channels (Chat/Command routing)
                    if let serenity::FullEvent::Message { new_message } = event {
                        // Ignore bot messages
                        if new_message.author.id != ctx.cache.current_user().id {
                            let channel_id = new_message.channel_id;
                            let content = new_message.content.clone();

                            if channel_id == data.chat_channel_id {
                                info!("💬 Routing message from chat channel to Core: {}", content);
                                let _ = data.cmd_tx.send(ControlCommand::Chat { 
                                    message: content, 
                                    channel_id: channel_id.get() 
                                }).await;
                            } else if channel_id == data.command_channel_id {
                                info!("⚙️ Routing message from command channel to Core: {}", content);
                                let _ = data.cmd_tx.send(ControlCommand::CommandChat { 
                                    message: content, 
                                    channel_id: channel_id.get() 
                                }).await;
                            }
                        }
                    }

                    // Handle approval buttons
                    if let serenity::FullEvent::InteractionCreate { interaction } = event {
                        if let Some(it) = interaction.as_message_component() {
                            if it.data.custom_id.starts_with("approve_") || it.data.custom_id.starts_with("reject_") {
                                let approved = it.data.custom_id.starts_with("approve_");
                                let uuid_str = it.data.custom_id.split('_').nth(1).unwrap_or("");
                                if let Ok(tid) = uuid::Uuid::parse_str(uuid_str) {
                                    let cmd = ControlCommand::ApprovalResponse { transition_id: tid, approved };
                                    let _ = data.cmd_tx.send(cmd).await;
                                    let _ = it.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .content(format!("{} **{}**", if approved { "✅ Approved" } else { "❌ Rejected" }, tid))
                                            .components(vec![])
                                    )).await;
                                }
                            }
                        }
                    }

                    // W-3: Handle 🔥/🗑️ reactions for Samsara evaluation
                    if let serenity::FullEvent::ReactionAdd { add_reaction } = event {
                        // Ignore bot's own reactions
                        if add_reaction.user_id.map(|u| u != ctx.cache.current_user().id).unwrap_or(false) {
                            let emoji = add_reaction.emoji.to_string();
                            let rating = match emoji.as_str() {
                                "🔥" => Some(1i32),
                                "🗑️" => Some(-1i32),
                                _ => None,
                            };
                            if let Some(r) = rating {
                                // Read the embed from the message to extract the job_id
                                if let Ok(msg) = add_reaction.channel_id.message(&ctx.http, add_reaction.message_id).await {
                                    if let Some(embed) = msg.embeds.first() {
                                        // Extract job_id from the "Job ID" field
                                        if let Some(field) = embed.fields.iter().find(|f| f.name == "Job ID") {
                                            let job_id = field.value.clone();
                                            let _ = data.cmd_tx.send(ControlCommand::SetCreativeRating { job_id: job_id.clone(), rating: r }).await;
                                            let _ = add_reaction.channel_id.say(&ctx.http, format!("🧘 **Karma Received**: Job {} rated {} by human.", job_id, if r > 0 { "🔥 (+1)" } else { "🗑️ (-1)" })).await;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Ok(())
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let cmd_tx_clone = cmd_tx.clone();
                let data = Data { 
                    cmd_tx, 
                    latest_status, 
                    log_channel_id: ChannelId::new(log_channel_id),
                    command_channel_id: ChannelId::new(command_channel_id),
                    chat_channel_id: ChannelId::new(chat_channel_id),
                };
                
                // Event Forwarder with Throttling + System Alert Channel
                let http = ctx.http.clone();
                let log_chan = data.log_channel_id;
                tokio::spawn(async move {
                    let mut buffer: Vec<LogEntry> = Vec::new();
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
                    loop {
                        tokio::select! {
                            Some(event) = event_rx.recv() => {
                                match event {
                                    CoreEvent::Log(l) => {
                                        buffer.push(l);
                                        if buffer.len() > 10 { // Flush if buffer large
                                            flush_logs(&mut buffer, log_chan, &http).await;
                                        }
                                    }
                                    CoreEvent::ApprovalRequest { transition_id, description } => {
                                        let msg = CreateMessage::new()
                                            .content(format!("🚨 **Approval Required**
{}", description))
                                            .button(CreateButton::new(format!("approve_{}", transition_id)).label("✅ Approve").style(serenity::ButtonStyle::Success))
                                            .button(CreateButton::new(format!("reject_{}", transition_id)).label("❌ Reject").style(serenity::ButtonStyle::Danger));
                                        let _ = log_chan.send_message(&http, msg).await;
                                    }
                                    CoreEvent::TaskCompleted { job_id, result, topic, style, .. } => {
                                        // W-3: Rich embed notification for completed jobs
                                        let is_success = result.to_lowercase().contains("success") || result.to_lowercase().contains("completed");
                                        let embed = CreateEmbed::new()
                                            .title(if is_success { "✅ Job Completed" } else { "❌ Job Failed" })
                                            .field("Topic", &topic, true)
                                            .field("Style", &style, true)
                                            .field("Job ID", &job_id, false)
                                            .field("Result", &result, false)
                                            .color(if is_success { 0x00FF41 } else { 0xFF003C })
                                            .footer(serenity::all::CreateEmbedFooter::new("React 🔥 = Best (+1) | 🗑️ = Trash (-1) | No reaction = Neutral (0) after 30min"));
                                        let msg = CreateMessage::new().embed(embed);
                                        if let Ok(sent) = log_chan.send_message(&http, msg).await {
                                            // Add reaction buttons
                                            let _ = sent.react(&http, ReactionType::Unicode("🔥".to_string())).await;
                                            let _ = sent.react(&http, ReactionType::Unicode("🗑️".to_string())).await;

                                            // Lazy Distillation: 30-minute timer for default positive
                                            let cmd_tx_lazy = cmd_tx_clone.clone();
                                            let job_id_lazy = job_id.clone();
                                            let msg_id = sent.id;
                                            let http_lazy = http.clone();
                                            let chan_lazy = log_chan;
                                            tokio::spawn(async move {
                                                tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
                                                // Check if human has reacted (fetch message, look for non-bot reactions)
                                                if let Ok(msg) = chan_lazy.message(&http_lazy, msg_id).await {
                                                    let has_human_reaction = msg.reactions.iter().any(|r| r.count > 1); // >1 means someone besides bot reacted
                                                    if !has_human_reaction {
                                                        // Default Positive: no reaction = neutral (0)
                                                        let _ = cmd_tx_lazy.send(ControlCommand::SetCreativeRating { job_id: job_id_lazy, rating: 0 }).await;
                                                        let _ = chan_lazy.say(&http_lazy, format!("🧘 **Lazy Distillation**: Job {} auto-rated 0 (neutral). No human feedback received.", msg_id)).await;
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    CoreEvent::ChatResponse { response, channel_id, resource_path } => {
                                        let chan = ChannelId::new(channel_id);
                                        if let Some(path) = resource_path {
                                            // TODO: Support resource attachment via serenity CreateMessage::add_file
                                            let _ = chan.say(&http, format!("{} (Resource: {})", response, path)).await;
                                        } else {
                                            let _ = chan.say(&http, response).await;
                                        }
                                    }
                                    CoreEvent::ProactiveTalk { message, channel_id } => {
                                        // If channel_id is 0, use default command channel
                                        let target_chan = if channel_id == 0 {
                                            data.command_channel_id
                                        } else {
                                            ChannelId::new(channel_id)
                                        };
                                        let _ = target_chan.say(&http, message).await;
                                    }
                                    CoreEvent::AgentStatsResponse(stats) => {
                                        let target_chan = data.command_channel_id;

                                        // ゲージ（ProgressBar）の生成関数
                                        let make_bar = |val: i32, max: i32| -> String {
                                            let full = (val as f32 / max as f32 * 10.0) as usize;
                                            let empty = 10 - full.min(10);
                                            format!("`{}{}` ({}%)", "■".repeat(full), "□".repeat(empty), (val as f32 / max as f32 * 100.0) as i32)
                                        };

                                        let embed = CreateEmbed::new()
                                            .title("📊 Watchtower Evolution Stats")
                                            .description(format!("Master, look! I'm growing every day... (Lv. {})", stats.level))
                                            .field("💖 Resonance (共鳴度)", make_bar(stats.resonance, 1000), false)
                                            .field("⚙️ Tech Level (技術Lv)", make_bar(stats.exp / 10, 500), false)
                                            .field("🎨 Creativity (創造性)", make_bar(stats.creativity, 1000), false)
                                            .field("🔋 Fatigue (疲労度)", make_bar(stats.fatigue, 100), false)
                                            .color(0xFF69B4) // Pinkish
                                            .footer(serenity::all::CreateEmbedFooter::new(format!("Last Updated: {}", chrono::Utc::now().to_rfc3339())));

                                        let _ = target_chan.send_message(&http, CreateMessage::new().embed(embed)).await;
                                    }
                                    _ => {}
                                }
                            }
                            // W-1 & W-4: System alerts from UDS loop and Heartbeat Sentinel
                            Some(alert_msg) = discord_rx.recv() => {
                                let _ = log_chan.say(&http, &alert_msg).await;
                            }
                            _ = interval.tick() => {
                                flush_logs(&mut buffer, log_chan, &http).await;
                            }
                        }
                    }
                });

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
        | serenity::GatewayIntents::MESSAGE_CONTENT;
    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.context("Serenity error")
}

async fn flush_logs(buffer: &mut Vec<LogEntry>, channel: ChannelId, http: &Arc<serenity::Http>) {
    if buffer.is_empty() { return; }
    let mut content = String::from("🗒️ **Core Logs**
```
");
    for log in buffer.drain(..) {
        let line = format!("[{}] {}
", log.level, log.message);
        if content.len() + line.len() > 1900 { // Discord limit
            content.push_str("```");
            let _ = channel.say(http, &content).await;
            content = String::from("```
");
        }
        content.push_str(&line);
    }
    content.push_str("```");
    let _ = channel.say(http, &content).await;
}
