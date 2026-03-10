/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use serde::{Deserialize, Serialize};

/// Biome プロトコルにおける基本メッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMessage {
    /// 送信元の公開鍵 (Base64)
    pub sender_pubkey: String,
    /// 宛先の公開鍵 (Base64)
    pub recipient_pubkey: String,
    /// 対話のトピックID (Dialogue ID)
    pub topic_id: String,
    /// メッセージ本文 (暗号化されている場合は暗号文)
    pub content: String,
    /// 送信者の Karma 状態を参照する CID (Merkle DAG)
    pub karma_root_cid: String,
    /// Ed25519 署名
    pub signature: String,
    /// Lamport Clock 等のロジカル時刻
    pub lamport_clock: u64,
    /// タイムスタンプ
    pub timestamp: String,
    /// 暗号化方式 ("none", "aes-256-gcm", etc.)
    /// Phase 20 MVP では "none" を使用。
    pub encryption: String,
}

/// 対話の要約・状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDialogue {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub last_message_at: String,
    pub message_count: u32,
    pub summary: Option<String>,
    pub status: DialogueStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DialogueStatus {
    Active,
    Archived,
    Blocked,
}
