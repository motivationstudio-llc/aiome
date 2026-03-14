/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # 経済活動インターフェース (Commerce Interface)
//!
//! AIエージェントが自律的に経済活動（決済、購入、報酬受取）を行うためのインターフェースを定義する。
//! このモジュールは `nurture` feature が有効な場合のみ機能する。

use crate::error::AiomeError;
use async_trait::async_trait;
use uuid::Uuid;

/// 経済エンジン・トレイト
///
/// `Project-Nurture` 等の商用モジュールによって実装される。
#[async_trait]
pub trait CommerceEngine: Send + Sync {
    /// エージェントの現在の残高（コイン数）を取得する
    async fn get_balance(&self, agent_id: Uuid) -> Result<u64, AiomeError>;

    /// 実施予定のアクションが経済ポリシー（予算、日次上限、安全性）に適合するか検証する
    async fn validate_activity(
        &self,
        agent_id: Uuid,
        activity_type: &str,
        amount: u64,
    ) -> Result<(), AiomeError>;

    /// 自律的な決済を実行する
    ///
    /// `item_id`: 購入対象の商標・アイテムID
    /// `metadata`: 決済に関連する追加情報
    async fn execute_autonomous_purchase(
        &self,
        agent_id: Uuid,
        item_id: Uuid,
        metadata: serde_json::Value,
    ) -> Result<String, AiomeError>; // 戻り値はトランザクションID
}

/// 経済コンテキスト
///
/// LLMのプロンプト等に注入するための、現在の経済状況サマリー。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EconomicContext {
    /// 利用可能なコイン残高
    pub balance: u64,
    /// 今日使用したコインの総額
    pub spent_today: u64,
    /// 1日の使用上限
    pub daily_limit: u64,
}
