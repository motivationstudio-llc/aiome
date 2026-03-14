/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # SLO (Service Level Objective) Engine
//!
//! SREプラクティスに基づくエラーバジェット管理と監視エンジン。
//! 期間ごとのエラー予算を追跡し、閾値超過時に警告を発する。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// SLO Engine の設定
pub struct SloConfig {
    /// 期間あたりの最大エラー数
    pub error_budget_max: usize,
    /// 警告を発する閾値
    pub warning_threshold: usize,
}

/// SLO Engine 本体
pub struct SloEngine {
    error_budget_consumed: Arc<AtomicUsize>,
    config: SloConfig,
    reset_time: Arc<RwLock<chrono::DateTime<chrono::Utc>>>,
    period_duration: chrono::Duration,
}

impl SloEngine {
    /// 新しい SloEngine を生成する
    pub fn new(config: SloConfig, period_duration: chrono::Duration) -> Self {
        Self {
            error_budget_consumed: Arc::new(AtomicUsize::new(0)),
            config,
            reset_time: Arc::new(RwLock::new(chrono::Utc::now() + period_duration)),
            period_duration,
        }
    }

    /// 期間の経過を確認し、必要に応じてバジェットをリセットする
    pub async fn check_period_reset(&self) {
        let mut reset_time = self.reset_time.write().await;
        let now = chrono::Utc::now();
        if now > *reset_time {
            self.error_budget_consumed.store(0, Ordering::Relaxed);
            *reset_time = now + self.period_duration;
            tracing::info!("SLO Engine: Error budget reset for new period.");
        }
    }

    /// エラーを記録し、バジェットの消費状況を更新する
    pub async fn record_error(&self) {
        self.check_period_reset().await;
        let consumed = self.error_budget_consumed.fetch_add(1, Ordering::Relaxed) + 1;

        if consumed == self.config.warning_threshold {
            warn!(
                "SLO Engine: Error budget approaching limit! Consumed: {}/{}",
                consumed, self.config.error_budget_max
            );
        } else if consumed >= self.config.error_budget_max {
            warn!(
                "SLO Engine: ERROR BUDGET EXHAUSTED! Consumed: {}/{}",
                consumed, self.config.error_budget_max
            );
        }
    }

    /// 現在のバジェット消費状況を返す (consumed, max)
    pub async fn get_budget_status(&self) -> (usize, usize) {
        self.check_period_reset().await;
        (
            self.error_budget_consumed.load(Ordering::Relaxed),
            self.config.error_budget_max,
        )
    }
}
