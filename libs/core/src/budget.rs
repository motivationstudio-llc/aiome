/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service,
 * where the service provides users with access to any substantial set of the features
 * or functionality of the software.
 */

use std::sync::atomic::{AtomicU64, Ordering};

/// 予算管理 (JobBudget):
/// LLMの暴走や外部APIの過剰呼び出しを物理的に遮断するための予算計上モジュール。
pub struct JobBudget {
    max_cost_microusds: u64,
    current_cost_microusds: AtomicU64,
}

impl JobBudget {
    /// 新しい予算枠を作成する (USDの1,000,000分の1単位)
    pub fn new(max_cost_usd: f64) -> Self {
        Self {
            max_cost_microusds: (max_cost_usd * 1_000_000.0) as u64,
            current_cost_microusds: AtomicU64::new(0),
        }
    }

    /// 費用を計上する。上限を超えた場合は Error を返す。
    pub fn charge(&self, cost_usd: f64) -> Result<(), BudgetExhaustedError> {
        let cost_micro = (cost_usd * 1_000_000.0) as u64;

        let mut current = self.current_cost_microusds.load(Ordering::SeqCst);
        loop {
            if current + cost_micro > self.max_cost_microusds {
                return Err(BudgetExhaustedError {
                    limit: self.max_cost_microusds as f64 / 1_000_000.0,
                    actual: (current + cost_micro) as f64 / 1_000_000.0,
                });
            }

            match self.current_cost_microusds.compare_exchange_weak(
                current,
                current + cost_micro,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Ok(()),
                Err(updated) => current = updated,
            }
        }
    }

    /// 現在の累計コストを取得する
    pub fn current_cost(&self) -> f64 {
        self.current_cost_microusds.load(Ordering::SeqCst) as f64 / 1_000_000.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("🚨 [JobBudget] 予算上限超過: limit=${limit:.4}, actual=${actual:.4}")]
pub struct BudgetExhaustedError {
    pub limit: f64,
    pub actual: f64,
}
