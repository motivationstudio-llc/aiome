/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

//! # ドメイントレイト定義
//!
//! Framework の4つのツールモジュールのインターフェースを定義する。
//! 具体実装は `libs/infrastructure` に配置する（依存性逆転の原則）。

use crate::error::AiomeError;
use crate::contracts::OracleVerdict;
use async_trait::async_trait;
use std::path::PathBuf;

/// トレンド調査ツール (TrendSonar)
///
/// X, Google Trends, 5ch 等から今バズっているテーマを取得する。
#[async_trait]
pub trait TrendSource: Send + Sync {
    /// 指定カテゴリのトレンドキーワードを取得
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError>;
}

/// トレンド情報の1件分
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrendItem {
    /// キーワード
    pub keyword: String,
    /// ソース (例: "X", "GoogleTrends", "5ch")
    pub source: String,
    /// スコア (高いほど注目度が高い)
    pub score: f64,
}

/// 生成エンジン (旧 GenerativeEngine)
#[async_trait]
pub trait GenerativeEngine: Send + Sync {
    /// ワークフローを実行し、生成されたファイルのパスを返す
    async fn generate_artifact(
        &self,
        prompt: &str,
        workflow_id: &str,
        input_artifact: Option<&std::path::Path>,
    ) -> Result<crate::contracts::ArtifactResponse, AiomeError>;

    /// 接続状態を確認
    async fn health_check(&self) -> Result<bool, AiomeError>;
}

/// メディアプロセッサー (旧 MediaForge)
#[async_trait]
pub trait MediaProcessor: Send + Sync {
    /// 複数のアセットを合成して最終出力を生成
    async fn combine_assets(
        &self,
        input: &PathBuf,
        context: &PathBuf,
        metadata: Option<&PathBuf>,
        force_style: Option<String>,
    ) -> Result<PathBuf, AiomeError>;

    /// メディアを標準化 (旧 standardize_format)
    async fn standardize_media(&self, input: &PathBuf) -> Result<PathBuf, AiomeError>;

    /// 複数のメディアブロックを 1つのファイルに結合
    async fn concatenate_media(&self, blocks: Vec<String>, output_name: String) -> Result<String, AiomeError>;

    /// メディアファイルの尺長（秒）を取得する
    async fn get_duration(&self, path: &std::path::Path) -> Result<f32, AiomeError>;
}

// --- Phase 10: The Automaton ---

/// ジョブステータス
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ToString for JobStatus {
    fn to_string(&self) -> String {
        match self {
            JobStatus::Pending => "Pending".to_string(),
            JobStatus::Processing => "Processing".to_string(),
            JobStatus::Completed => "Completed".to_string(),
            JobStatus::Failed => "Failed".to_string(),
        }
    }
}

impl JobStatus {
    pub fn from_string(s: &str) -> Self {
        match s {
            "Processing" => JobStatus::Processing,
            "Completed" => JobStatus::Completed,
            "Failed" => JobStatus::Failed,
            _ => JobStatus::Pending,
        }
    }
}

/// 永続化ジョブ (The Immortal Schema)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub id: String,
    pub category: String,
    pub topic: String,
    pub style: String,
    /// LLM Structured Output (KarmaDirectives) をJSON文字列として格納
    pub karma_directives: Option<String>,
    pub status: JobStatus,
    /// ゾンビタスク回収のための実行開始時刻
    pub started_at: Option<String>,
    /// The Heartbeat Pulse: 長時間レンダリング中のワーカー生存証明
    pub last_heartbeat: Option<String>,
    /// 技術的教訓の自動抽出が完了したか
    pub tech_karma_extracted: bool,
    /// クリエイティブ評価 (人間からの非同期評価): -1=ボツ, 0=普通, 1=最高, None=未評価
    pub creative_rating: Option<i32>,
    /// Log-First Distillation: 実行ログを永続化し、LLMダウン時でも後から蒸留可能にする
    pub execution_log: Option<String>,
    pub error_message: Option<String>,
    // --- Phase 11: World-in-the-Loop SNS Integration ---
    pub sns_platform: Option<String>,
    pub sns_content_id: Option<String>,
    pub published_at: Option<String>,
    /// 多言語出力された成果物のリスト (JSON文字列)
    pub output_artifacts: Option<String>,
}

/// ジョブキュー (The Persistent Memory & Samsara)
///
/// SQLite等を用いた非同期ジョブ管理とKarmaの抽出・記録を行う。
/// The Immortal Schema に準拠。
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// 新規ジョブをキューに追加 (Pending)
    async fn enqueue(&self, category: &str, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, AiomeError>;

    /// 指定したIDのジョブを取得する
    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError>;

    /// 次に実行すべき Pending ジョブを 1件取得し、Processing に更新
    async fn dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError>;

    /// ジョブを完了状態にする
    async fn complete_job(&self, job_id: &str, output_artifacts: Option<&str>) -> Result<(), AiomeError>;

    /// ジョブを失敗状態にする
    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError>;

    // --- Phase 10-A.5 The Samsara Protocol ---
    /// RAG-Driven Karma Injection: トピックとSkillIDに関連する過去の教訓を抽出する
    async fn fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64, current_soul_hash: &str) -> Result<Vec<String>, AiomeError>;

    /// 抽出された教訓（Karma）を保存する
    /// `karma_type`: 'Technical', 'Creative', 'Synthesized'
    async fn store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str, soul_hash: &str) -> Result<(), AiomeError>;

    /// The Zombie Hunter: 一定時間以上 Processing のまま放置されたジョブを Failed に強制移行する
    /// Heartbeat 版: last_heartbeat が timeout 分以上途絶えているものを回収
    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError>;

    /// クリエイティブ評価 (人間からの非同期フィードバック) を設定する
    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError>;

    /// The Heartbeat Pulse: 長時間処理中のワーカーが生存を証明する
    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError>;

    /// Log-First Distillation: 実行ログをDBに永続化する（LLMダウン時でも教訓を失わない）
    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError>;

    /// すべての教訓（Karma）を最新順に取得する
    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;

    /// Deferred Distillation: ログはあるが Karma 未抽出のジョブを検索する
    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;

    /// Distillation完了マーク: tech_karma_extracted = 1 にセットする
    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError>;

    /// DB Scavenger: 指定日数以上経過した Completed/Failed ジョブを物理削除する。
    /// karma_logs は `ON DELETE SET NULL` により孤立しても保持される (Eternal Karma)。
    /// 戻り値は削除されたジョブ数。
    async fn purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError>;

    /// SNSコンテンツIDをジョブに紐付ける (Phase 11: The Anchor Link)
    async fn link_sns_data(&self, job_id: &str, platform: &str, content_id: &str) -> Result<(), AiomeError>;

    /// 評価マイルストーンに到達した未評価のジョブを取得する (Phase 11: The Catch-up Logic)
    async fn fetch_jobs_for_evaluation(&self, milestone_days: i64, limit: i64) -> Result<Vec<Job>, AiomeError>;

    /// 取得したSNSメトリクスを台帳に記録する (Phase 11: The Metrics Ledger)
    #[allow(clippy::too_many_arguments)]
    async fn record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError>;

    /// 評価待ち（Oracle未実行）のメトリクス履歴を取得する (Phase 11: Evaluate Phase)
    async fn fetch_pending_evaluations(&self, limit: i64) -> Result<Vec<SnsMetricsRecord>, AiomeError>;

    /// Oracleの評価を適用し、業（Karma）を更新・台帳を完了させる (Phase 11: Commit Phase)
    /// 「台帳の完了」と「業の永続化」を単一トランザクションで行う冪等なアトミック操作。
    async fn apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError>;

    /// 最近のジョブをN件取得する
    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;

    // --- Phase 12: Evolutionary Dynamics ---
    /// 育成ステータスを取得
    async fn get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError>;
    /// 共鳴度を加算 (Chat対応等)
    async fn add_resonance(&self, amount: i32) -> Result<(), AiomeError>;
    /// 技術経験値を加算 (Samsara完了等)
    async fn add_tech_exp(&self, amount: i32) -> Result<(), AiomeError>;
    /// 創造性を加算
    async fn add_creativity(&self, amount: i32) -> Result<(), AiomeError>;
    /// Samsaraレベルを同期 (成長計算の実行)
    async fn sync_samsara_level(&self) -> Result<Option<crate::contracts::SamsaraEvent>, AiomeError>;

    // --- Biome Dialogue Expansion ---
    /// 対象トピックの現在のターン数とクールダウン期限を取得
    async fn get_biome_topic_status(&self, topic_id: &str) -> Result<Option<(i32, Option<String>)>, AiomeError>;
    /// トピックを1ターン進め、クールダウンを設定する
    async fn advance_biome_turn(&self, topic_id: &str, cooldown_minutes: i64) -> Result<i32, AiomeError>;
    /// 指定Pubkeyの評判（Reputation）を更新する
    async fn update_biome_reputation(&self, pubkey: &str, delta: f64) -> Result<f64, AiomeError>;

    /// 進化イベントの記録 (Evolution Chronicle)
    async fn record_evolution_event(&self, level: i32, event_type: &str, description: &str, inspiration: Option<&str>, karma_json: Option<&str>) -> Result<(), AiomeError>;
    /// 進化履歴の取得
    async fn fetch_evolution_history(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;

    /// 保留中（Pending）のジョブ数を取得する
    async fn get_pending_job_count(&self) -> Result<i64, AiomeError>;

    /// 指定した時刻以降に作成されたジョブ数を取得する
    async fn get_job_count_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError>;

    /// SNSで再生数上位のジョブを取得する (成功パターンの分析用)
    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;

    /// 魂の変異履歴を記録する (Phase 5: Transmigration)
    async fn record_soul_mutation(&self, old_hash: &str, new_hash: &str, reason: &str) -> Result<(), AiomeError>;

    /// 魂の書き換え（Transmigration）に使用する未反映の教訓を取得する
    async fn fetch_unincorporated_karma(&self, limit: i64, current_soul_hash: &str) -> Result<Vec<serde_json::Value>, AiomeError>;

    /// 教訓が魂に反映されたことを記録し、新世代ハッシュを付与する
    async fn mark_karma_as_incorporated(&self, karma_ids: Vec<String>, new_soul_hash: &str) -> Result<(), AiomeError>;

    /// 現在のリトライ回数を取得
    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError>;
    /// リトライ回数をインクリメント。毒薬発動(Failed移行)した場合は true を返す
    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError>;
    /// リトライ回数をリセット
    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError>;

    // --- Phase 12-C: Immune & Arena ---
    async fn store_immune_rule(&self, rule: &crate::contracts::ImmuneRule) -> Result<(), AiomeError>;
    async fn fetch_active_immune_rules(&self) -> Result<Vec<crate::contracts::ImmuneRule>, AiomeError>;
    async fn record_arena_match(&self, match_data: &crate::contracts::ArenaMatch) -> Result<(), AiomeError>;

    // --- Phase 12-F: Karma Federation ---
    /// Federation: 外部ノードへ提供するためのデータを取得
    async fn export_federated_data(&self, since: Option<&str>) -> Result<(Vec<crate::contracts::FederatedKarma>, Vec<crate::contracts::ImmuneRule>, Vec<crate::contracts::ArenaMatch>), AiomeError>;
    
    /// Federation: 外部ノードから受け取ったデータをUPSERTで取り込む
    async fn import_federated_data(&self, karmas: Vec<crate::contracts::FederatedKarma>, rules: Vec<crate::contracts::ImmuneRule>, matches: Vec<crate::contracts::ArenaMatch>) -> Result<(), AiomeError>;
    
    /// Federation: 宛先ノード(Peer)ごとの最終同期時刻を取得・更新
    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError>;
    async fn update_peer_sync_time(&self, peer_url: &str, sync_time: &str) -> Result<(), AiomeError>;
    
    /// Get all immune rules for visualization
    async fn get_immune_rules(&self) -> Result<Vec<crate::contracts::ImmuneRule>, AiomeError>;

    // --- Phase 10-B: Swarm Logic ---
    /// Get the node's unique ID (Public Key)
    async fn get_node_id(&self) -> Result<String, AiomeError>;
    /// Sign a message payload for the Swarm
    async fn sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError>;
    /// Increment the local Lamport clock and return new value
    async fn tick_local_clock(&self) -> Result<u64, AiomeError>;
    /// Update local clock based on a received remote clock value
    async fn sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError>;
    
    /// The Scavenger: Storage GC (Remove old artifact files if total size > threshold_gb)
    async fn storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError>;

    // --- Chat & Memory (The Soul Persistence) ---
    async fn store_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), AiomeError>;
    async fn fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
}

/// コンテンツ・パブリッシャー (Publishing Engine)
#[async_trait]
pub trait Publisher: Send + Sync {
    /// SNS やブログ等のプラットフォームに投稿する
    async fn publish(&self, content: &str, media_paths: &[PathBuf], metadata: &serde_json::Value) -> Result<String, AiomeError>;
    
    /// プラットフォーム名を取得 (例: "Twitter", "LocalBlog", "MockX")
    fn platform_name(&self) -> &str;
}

/// 相互監視型 LLM バリデーター (The Constitutional Prosecutor)
#[async_trait]
pub trait ConstitutionalValidator: Send + Sync {
    /// 生成された内容が魂の原則（SOUL.md）に準拠しているか検証する
    async fn verify_constitutional(&self, content: &str, principles: &str) -> Result<(), AiomeError>;
}


/// 評価台帳（sns_metrics_history）のレコード構造体
#[derive(Debug, Clone)]
pub struct SnsMetricsRecord {
    pub id: i64,
    pub job_id: String,
    pub milestone_days: i64,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
    pub raw_comments_json: Option<String>,
    pub hard_metric_score: Option<f64>,
    pub engagement_rate: Option<f64>,
}


/// ログ・通知ツール (AiomeLog)
///
/// 稼働ログをSQLiteに記録し、必要に応じてSlack/Discordに通知する。
#[async_trait]
pub trait AiomeLogger: Send + Sync {
    /// 成功ログ
    async fn log_success(&self, artifact_id: &str, output_path: &std::path::PathBuf) -> Result<(), AiomeError>;

    /// エラーをログに記録
    async fn log_error(&self, reason: &str) -> Result<(), AiomeError>;

    /// 日次サマリーを生成
    async fn daily_summary(&self, _jail: &bastion::fs_guard::Jail) -> Result<String, AiomeError>;
}

/// [法定義] 第1条 & 第2条：物理的境界と通信プロトコル
///
/// すべての AI アクターが遵守すべき基本インターフェース。
/// 物理的なリソースにアクセスする際は、必ず Jail（檻）を介さなければならない。
#[async_trait]
pub trait AgentAct: Send + Sync {
    type Input: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Clone;
    type Output: serde::Serialize + for<'de> serde::Deserialize<'de> + Send;

    /// 憲法第1条に従い、Jail ハンドルを必須とする実行メソッド
    /// 
    /// Runtime Verification (Design by Contract):
    /// 実行前と実行後の状態遷移は実装側のSDK境界マクロで強制されます。
    async fn execute(
        &self,
        input: Self::Input,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, AiomeError>;
}
