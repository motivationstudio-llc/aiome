/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # Circuit Breaker パターン実装
//!
//! LLM呼び出しや外部サービス連携における障害伝播を防ぐための
//! Circuit Breaker パターンの実装。状態管理と指数バックオフ的
//! フェイルファーストを提供する。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Circuit Breaker の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 正常稼働中
    Closed,
    /// 障害検知により遮断中（即座にエラー返却）
    Open,
    /// 復旧テスト中（次の1回で判定）
    HalfOpen,
}

/// Circuit Breaker の設定
pub struct CircuitBreakerConfig {
    /// Open 状態に遷移するまでの連続失敗数
    pub failure_threshold: usize,
    /// Open → HalfOpen に遷移するまでの待機時間
    pub reset_timeout: std::time::Duration,
}

/// Circuit Breaker 本体
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failures: Arc<AtomicUsize>,
    config: CircuitBreakerConfig,
    last_failure_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl CircuitBreaker {
    /// 新しい CircuitBreaker を生成する
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failures: Arc::new(AtomicUsize::new(0)),
            config,
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    /// 現在の状態をチェックし、リクエストを通すべきか判定する
    pub async fn check_state(&self) -> Result<(), &'static str> {
        let mut state = self.state.write().await;

        if *state == CircuitState::Open {
            let last_fail = *self.last_failure_time.read().await;
            if let Some(time) = last_fail {
                if time.elapsed() > self.config.reset_timeout {
                    tracing::info!("CircuitBreaker: Half-Open state entered. Testing service.");
                    *state = CircuitState::HalfOpen;
                    return Ok(());
                }
            }
            return Err("CircuitBreaker is OPEN. Failing fast.");
        }
        Ok(())
    }

    /// 成功を記録し、HalfOpen なら Closed に復旧する
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        if *state == CircuitState::HalfOpen {
            tracing::info!("CircuitBreaker: Service recovered. State -> Closed.");
            *state = CircuitState::Closed;
            self.failures.store(0, Ordering::Relaxed);
        } else {
            self.failures.store(0, Ordering::Relaxed);
        }
    }

    /// 失敗を記録し、閾値超過なら Open に遷移する
    pub async fn record_failure(&self) {
        let fails = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        let mut state = self.state.write().await;

        if *state == CircuitState::HalfOpen || fails >= self.config.failure_threshold {
            if *state != CircuitState::Open {
                tracing::warn!("CircuitBreaker: Threshold reached. State -> Open.");
                *state = CircuitState::Open;
            }
            let mut last_fail = self.last_failure_time.write().await;
            *last_fail = Some(std::time::Instant::now());
        }
    }
}
