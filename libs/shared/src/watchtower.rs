/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service,
 * where the service provides users with access to any substantial set of the features
 * or functionality of the software.
 */

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_used_mb: u64,
    pub vram_used_mb: u64,
    pub active_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub target: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    Log(LogEntry),
    Heartbeat(SystemStatus),
    ApprovalRequest {
        transition_id: Uuid,
        description: String,
    },
    TaskCompleted {
        job_id: String,
        result: String,
        topic: String,
        style: String,
        preview_url: Option<String>,
    },
    /// コアからの対話応答 (音声付き)
    ChatResponse {
        response: String,
        channel_id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource_path: Option<String>,
    },
    /// 自律的な話しかけ（プッシュ通知）
    ProactiveTalk {
        message: String,
        channel_id: u64,
    },
    /// 育成ステータスの応答
    AgentStatsResponse(AgentStats),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentStats {
    pub level: i32,
    pub exp: i32,
    pub resonance: i32,
    pub creativity: i32,
    pub fatigue: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    GetStatus,
    /// 育成ステータス取得
    GetAgentStats,
    /// Aiomeとの対話 (一般チャット)
    Chat {
        message: String,
        channel_id: u64,
    },
    /// システム操作用の対話 (コマンドチャネル)
    CommandChat {
        message: String,
        channel_id: u64,
    },
    Generate {
        category: String,
        topic: String,
        style: Option<String>,
    },
    StopGracefully,
    /// Hybrid Nuke Protocol: 即時強制終了要求
    EmergencyShutdown,
    ApprovalResponse {
        transition_id: Uuid,
        approved: bool,
    },
    /// Samsara Phase 4: 人間からのクリエイティブ評価
    SetCreativeRating {
        job_id: String,
        rating: i32,
    },
    /// Phase 11: The Anchor Link (SNS動画IDの紐付け)
    LinkSns {
        job_id: String,
        platform: String,
        content_id: String,
    },
}
