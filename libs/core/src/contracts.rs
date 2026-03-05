/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # The Contract — アクター間通信契約
//!
//! 憲法第2条に基づき、アクター間のやり取りを型安全に定義する。

use serde::{Deserialize, Serialize};
use crate::traits::TrendItem;

/// 監査用メタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMeta {
    pub trace_id: String,
    pub sender_id: String,
}

/// メッセージの基本構造
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<T> {
    pub meta: MessageMeta,
    pub payload: T,
}

// --- Trend クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendRequest {
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendResponse {
    pub items: Vec<TrendItem>,
}

// --- Concept クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRequest {
    pub topic: String,
    pub category: String,
    pub trend_items: Vec<TrendItem>,
    /// 利用可能な演出スタイルの一覧
    pub available_styles: Vec<String>,
    
    // --- Phase 12-B: Karmic Supervision ---
    /// 過去の教訓 (Karma) のリスト
    #[serde(default)]
    pub relevant_karma: Vec<String>,
    /// 前回の試行で失敗した際の実行ログ
    pub previous_attempt_log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedScript {
    pub lang: String,
    pub display_intro: String,
    pub display_body: String,
    pub display_outro: String,
    pub script_intro: String,
    pub script_body: String,
    pub script_outro: String,
    #[serde(default)]
    pub style_intro: String,
    #[serde(default)]
    pub style_body: String,
    #[serde(default)]
    pub style_outro: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptResponse {
    pub title: String,
    /// 字幕表示用テキスト（英数字・記号をそのまま使用）
    #[serde(default)]
    pub display_intro: String,
    #[serde(default)]
    pub display_body: String,
    #[serde(default)]
    pub display_outro: String,
    /// 導入部 (backward compatibility)
    #[serde(default)]
    pub script_intro: String,
    /// 本編 (backward compatibility)
    #[serde(default)]
    pub script_body: String,
    /// 結末 (backward compatibility)
    #[serde(default)]
    pub script_outro: String,
    
    #[serde(default)]
    pub style_intro: String,
    #[serde(default)]
    pub style_body: String,
    #[serde(default)]
    pub style_outro: String,
    
    /// 多言語化された台本リスト
    #[serde(default)]
    pub scripts: Vec<LocalizedScript>,

    /// 全体共通の画風、ライティング、特定のキャラクター指定 (Subject/Style)
    pub common_style: String,
    /// 採択された演出スタイル (styles.toml のキー)
    pub style_profile: String,
    /// 各シーン固有の描写 (Action/Background) - 必ず3件
    pub visual_prompts: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

// --- Generative Engine クラスター (旧 Video) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeRequest {
    pub prompt: String,
    pub workflow_id: String,
    pub input_artifact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactResponse {
    pub output_path: String,
    pub job_id: String,
}

// --- Voice クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceRequest {
    pub text: String,
    pub voice: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// 音声の言語 (ja, en等)
    #[serde(default)]
    pub lang: Option<String>,
    /// 感情スタイル (Neutral, Happy, Sad, Angry等)
    #[serde(default)]
    pub style: Option<String>,
    /// モデルディレクトリ名 (Noneの場合はデフォルト)
    #[serde(default)]
    pub model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceResponse {
    pub audio_path: String,
}

// --- Media クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProcessingRequest {
    pub input_path: String,
    pub context_path: Option<String>,
    pub metadata_path: Option<String>,
    pub force_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProcessingResponse {
    pub final_path: String,
}

// --- Workflow クラスター (Phase 5) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStyle {
    // --- 視覚演出 (Cameraman) ---
    pub zoom_speed: Option<f64>,
    pub pan_intensity: Option<f64>,
    
    // --- 音響演出 (SoundMixer) ---
    pub bgm_volume: Option<f32>,
    pub ducking_threshold: Option<f32>,
    pub ducking_ratio: Option<f32>,
    pub fade_duration: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputArtifact {
    pub tag: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub category: String,
    pub topic: String,
    /// Remix 対象のコンテンツID (None の場合は新規作成)
    pub remix_id: Option<String>,
    /// スキップ先のステップ (None の場合はフル実行)
    pub skip_to_step: Option<String>,
    
    // --- Phase 8.5 Remix Lab Extensions ---
    /// 適用するスタイル名 (styles.toml のキー)
    #[serde(default)]
    pub style_name: String,
    /// ユーザーによるカスタム調整 (None の場合はプリセット通り)
    pub custom_style: Option<CustomStyle>,

    /// 生成対象言語 (例: ["ja", "en"])
    #[serde(default)]
    pub target_langs: Vec<String>,

    // --- Phase 12-B: Karmic Supervision ---
    /// 過去の教訓 (Karma) のリスト
    #[serde(default)]
    pub relevant_karma: Vec<String>,
    /// 前回の試行で失敗した際の実行ログ
    pub previous_attempt_log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub final_artifact_path: String,
    /// 生成された成果物のリスト
    #[serde(default)]
    pub output_artifacts: Vec<OutputArtifact>,
    pub concept: ConceptResponse,
}

// --- Phase 10-F: The Absolute Contract v2 (最終確定・Rust構造体) ---

/// LLMに要求する、本日のタスク生成の「全体レスポンス」。
/// `topic` と `style` は DB の独立カラムへ、`directives` は JSON カラムへ分離格納される。
/// (The Split Payload — データの二重化防止)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmJobResponse {
    /// コンテンツの具体的なテーマ (DB `jobs.topic` カラムへ直接マッピング)
    pub topic: String,

    /// 使用するワークフロー (DB `jobs.style_name` カラムへ直接マッピング)
    /// ※Rust側で INSERT 前にファイルの実在チェック (Skill Existence Validation) を行うこと！
    pub style: String,

    /// DB `jobs.karma_directives` カラム (JSON) に格納される純粋な指示群
    pub directives: KarmaDirectives,
}

/// The strict JSON contract for the LLM output.
/// DB の `karma_directives` カラムに JSON 文字列として格納される「純粋な指示書」。
/// `CHECK(json_valid(karma_directives))` と連携し、不正な JSON を DB レイヤーで物理的に弾く。
///
/// # Design Decisions (The Payload Audit)
/// - `topic`/`style` は含まない（Split Payload: DB カラムと JSON の二重化防止）
/// - `parameter_overrides` は二重 HashMap（Node-Targeted Overrides: ComfyUI ノード狙い撃ち）
/// - `confidence_score` は `u8` だが、DB 挿入前に `.clamped()` で 0-100 に強制（Bounded Clamp）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KarmaDirectives {
    /// プロンプトへの追加指示 (Karmaから導出)
    #[serde(default)]
    pub positive_prompt_additions: String,

    /// NGワードや避けるべき表現
    #[serde(default)]
    pub negative_prompt_additions: String,

    /// ComfyUI ノードを正確に狙い撃ちするための二重階層マップ (Node-Targeted Overrides)
    /// 構造: { "NodeTitle": { "parameter_name": value } }
    /// 例: { "[API_SAMPLER]": { "cfg": 8.0, "denoise": 0.65 } }
    #[serde(default)]
    pub parameter_overrides: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,

    /// 過去のKarmaから導き出された、全体的な注意事項
    #[serde(default)]
    pub execution_notes: String,

    /// LLM 自身のこの生成に対する自信度 (0-100)。
    /// DB挿入前に必ず `.clamped()` を呼び出すこと。
    pub confidence_score: u8,
}

impl KarmaDirectives {
    /// The Bounded Clamp: u8 (max 255) と SQLite CHECK(weight BETWEEN 0 AND 100) の衝突を防ぐ安全弁。
    /// LLMの出力を信用せず、物理的に 0-100 の範囲に強制する。
    pub fn clamped_confidence(&self) -> u8 {
        self.confidence_score.clamp(0, 100)
    }
}

// --- Phase 11: The Absolute Contract v3 (神託の契約) ---

/// LLM（The Oracle）によるコンテンツの最終審判。
/// 大衆の反応（Engagement）と設計者の美学（Soul）を統合し、次世代への「業（Karma）」を導き出す。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleVerdict {
    /// 魂の整合性 (0.0 〜 1.0)
    pub alignment_score: f64,
    /// 成長への寄与 (0.0 〜 1.0)
    pub growth_score: f64,
    /// 次回への教訓
    pub lesson: String,
    /// 自己進化を試行すべきか
    pub should_evolve: bool,
    /// 内部推論
    pub reasoning: String,
}

// --- Phase 12-C: Adaptive Immune System & Skill Arena ---

/// 自己防衛のための免疫ルール
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmuneRule {
    pub id: String,
    /// 検知パターンの記述 (自然言語または正規表現)
    pub pattern: String,
    /// ルールの重要度 (1-100)
    pub severity: u8,
    /// 適用するアクション (Block, Warn, Quarantine)
    pub action: String,
    pub created_at: String,
}

/// 競争的淘汰アリーナの対戦記録
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaMatch {
    pub id: String,
    pub skill_a: String,
    pub skill_b: String,
    pub topic: String,
    /// 勝利したスキル名
    pub winner: Option<String>,
    pub reasoning: String,
    pub created_at: String,
}

// --- Phase 12-F: Karma Federation ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSyncRequest {
    /// このノードの一意識別子 (起動時に生成されたUUID)。Sybil攻撃対策。
    pub node_id: String,
    /// 前回の同期日時 (ISO8601等)。初回は None
    pub since: Option<String>,
    /// プロトコルバージョン。後方互換性のために使用。
    pub protocol_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSyncResponse {
    pub new_karmas: Vec<FederatedKarma>,
    pub new_immune_rules: Vec<ImmuneRule>,
    pub new_arena_matches: Vec<ArenaMatch>,
    /// 同期時点のレスポンス側サーバー時刻
    pub server_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedKarma {
    pub id: String,
    pub job_id: Option<String>,
    pub karma_type: String,
    pub related_skill: String,
    pub lesson: String,
    pub weight: i32,
    pub created_at: String,
    pub soul_version_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPushRequest {
    pub node_id: String,
    pub karmas: Vec<FederatedKarma>,
    pub rules: Vec<ImmuneRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPushResponse {
    pub accepted_count: usize,
    pub message: String,
}
