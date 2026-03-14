/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::error::AiomeError;
use sqlx::SqlitePool;
use tracing::info;

/// Karma Tiering Maintenance ジョブ (Adaptive Intelligence)
/// 7日/30日/90日のディケイ・サイクルに基づき tier を自動遷移させる。
pub async fn run_karma_tier_maintenance(pool: &SqlitePool) -> Result<(), AiomeError> {
    info!("🧬 [KarmaMaintenance] Starting Karma tiering cycle...");

    // 1. HOT 昇格: 直近7日間に3回以上適用された WARM なカルマ
    let hot_records = sqlx::query(
        "UPDATE karma_logs SET tier = 'HOT' 
         WHERE tier = 'WARM' 
         AND apply_count >= 3 
         AND last_applied_at > datetime('now', '-7 days')
         AND is_archived = 0",
    )
    .execute(pool)
    .await
    .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to promote to HOT: {}", e),
    })?
    .rows_affected();

    // 2. WARM 降格: 30日以上未使用の HOT なカルマ
    let warm_records = sqlx::query(
        "UPDATE karma_logs SET tier = 'WARM' 
         WHERE tier = 'HOT' 
         AND (last_applied_at IS NULL OR last_applied_at < datetime('now', '-30 days'))",
    )
    .execute(pool)
    .await
    .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to demote to WARM: {}", e),
    })?
    .rows_affected();

    // 3. COLD 降格 (アーカイブ): 90日以上未使用の WARM なカルマ
    let cold_records = sqlx::query(
        "UPDATE karma_logs SET tier = 'COLD', is_archived = 1 
         WHERE tier = 'WARM' 
         AND (last_applied_at IS NULL OR last_applied_at < datetime('now', '-90 days'))",
    )
    .execute(pool)
    .await
    .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to demote to COLD: {}", e),
    })?
    .rows_affected();

    if hot_records > 0 || warm_records > 0 || cold_records > 0 {
        info!(
            "✅ [KarmaMaintenance] Cycle completed: HOT promoted={}, WARM demoted={}, COLD archived={}",
            hot_records, warm_records, cold_records
        );
    }

    Ok(())
}
