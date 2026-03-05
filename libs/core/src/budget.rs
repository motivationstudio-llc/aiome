/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
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
        let prev = self.current_cost_microusds.fetch_add(cost_micro, Ordering::SeqCst);
        
        if prev + cost_micro > self.max_cost_microusds {
            Err(BudgetExhaustedError {
                limit: self.max_cost_microusds as f64 / 1_000_000.0,
                actual: (prev + cost_micro) as f64 / 1_000_000.0,
            })
        } else {
            Ok(())
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
