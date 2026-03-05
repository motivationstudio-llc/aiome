/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! ドメインエラー型
//!
//! `thiserror` を使い、すべてのドメインエラーに明確な型を付与する。
//! Iron Principles: `unwrap()` / `expect()` は禁止。

use thiserror::Error;

/// ShortsFactory のドメインエラー
#[derive(Debug, Error)]
pub enum FactoryError {
    // === コンテキスト調査 (旧 トレンド調査) ===
    #[error("コンテキスト取得に失敗: {source}")]
    ContextFetch {
        #[source]
        source: anyhow::Error,
    },

    // === 生成エンジン (旧 動画生成) ===
    #[error("外部サービス接続エラー (url: {url}): {source}")]
    RemoteServiceError {
        url: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("外部サービス実行タイムアウト ({timeout_secs}秒)")]
    RemoteServiceTimeout { timeout_secs: u64 },

    #[error("外部サービス実行失敗: {reason}")]
    RemoteServiceExecutionFailed { reason: String },

    // === 外部プロセッサー (旧 メディア編集) ===
    #[error("外部プロセス実行エラー: {reason}")]
    SubprocessFailed { reason: String },

    #[error("アーティファクトが見つからない: {path}")]
    ArtifactNotFound { path: String },

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
    #[error("リソース不足: 必要 {required_mb}MB, 利用可能 {available_mb}MB")]
    ResourceShortage {
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

    #[error("生成インターフェース失敗: {reason}")]
    GenerativeInterfaceError { reason: String },

    #[error("セキュリティ法規違反: {reason}")]
    SecurityViolation { reason: String },

    #[error("予算上限超過 (Budget Exhausted): {0}")]
    BudgetExhausted(#[from] crate::budget::BudgetExhaustedError),

    #[error("名誉ある撤退 (Honorable Abort): {reason}")]
    HonorableAbort { reason: String },
}
