/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # ドメインエラー型
//!
//! `thiserror` を使い、すべてのドメインエラーに明確な型を付与する。
//! Iron Principles: `unwrap()` / `expect()` は禁止。

use thiserror::Error;

/// ShortsFactory のドメインエラー
#[derive(Debug, Error)]
pub enum FactoryError {
    // === トレンド調査 ===
    #[error("トレンド取得に失敗: {source}")]
    TrendFetch {
        #[source]
        source: anyhow::Error,
    },

    // === 動画生成 ===
    #[error("ComfyUI 接続エラー (url: {url}): {source}")]
    ComfyConnection {
        url: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("ComfyUI ワークフロー実行タイムアウト ({timeout_secs}秒)")]
    ComfyTimeout { timeout_secs: u64 },

    #[error("ComfyUI ワークフロー実行失敗: {reason}")]
    ComfyWorkflowFailed { reason: String },

    // === メディア編集 ===
    #[error("FFmpeg 実行エラー: {reason}")]
    FfmpegFailed { reason: String },

    #[error("メディアファイルが見つからない: {path}")]
    MediaNotFound { path: String },

    // === ログ・通知 ===
    #[error("ログ記録エラー: {source}")]
    LogWrite {
        #[source]
        source: anyhow::Error,
    },

    // === LLM ===
    #[error("LLM 応答エラー: {source}")]
    LlmResponse {
        #[source]
        source: anyhow::Error,
    },

    #[error("Guardrails がプロンプトをブロック: {reason}")]
    PromptBlocked { reason: String },

    // === 設定 ===
    #[error("設定ファイル読み込みエラー: {source}")]
    ConfigLoad {
        #[source]
        source: anyhow::Error,
    },

    // === 運用・リソース管理 ===
    #[error("VRAM不足: 必要 {required_mb}MB, 利用可能 {available_mb}MB")]
    InsufficientVram {
        required_mb: u64,
        available_mb: u64,
    },

    #[error("ストレージ不足: 使用率が閾値 {threshold}% を超過")]
    StorageFull { threshold: f32 },

    #[error("運用タイムアウト: {reason}")]
    OperationalTimeout { reason: String },

    #[error("OSエラー: {source}")]
    OsError {
        #[source]
        source: anyhow::Error,
    },

    #[error("インフラ構造エラー: {reason}")]
    Infrastructure { reason: String },

    #[error("音声合成失敗 (TTS): {reason}")]
    TtsFailure { reason: String },

    #[error("セキュリティ法規違反: {reason}")]
    SecurityViolation { reason: String },

    #[error("名誉ある撤退 (Honorable Abort): {reason}")]
    HonorableAbort { reason: String },
}
