/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillPerformance {
    pub success_count: u64,
    pub failure_count: u64,
    pub average_latency_ms: u64,
    pub total_karma_weight: f64,
}

pub struct SkillArena {
    performance_map: Arc<RwLock<HashMap<String, SkillPerformance>>>,
    culling_threshold: f64, // e.g. 0.3 (failure rate)
}

impl SkillArena {
    pub fn new() -> Self {
        Self {
            performance_map: Arc::new(RwLock::new(HashMap::new())),
            culling_threshold: 0.5,
        }
    }

    /// [A-3] Skill Culling
    /// Record the outcome of a skill execution to update its reputation.
    pub async fn record_outcome(
        &self,
        skill_name: &str,
        is_success: bool,
        latency_ms: u64,
        karma_delta: f64,
    ) {
        let mut map = self.performance_map.write().await;
        let perf = map
            .entry(skill_name.to_string())
            .or_insert(SkillPerformance {
                success_count: 0,
                failure_count: 0,
                average_latency_ms: 0,
                total_karma_weight: 0.0,
            });

        if is_success {
            perf.success_count += 1;
        } else {
            perf.failure_count += 1;
        }

        perf.total_karma_weight += karma_delta;

        // Rolling average for latency
        let total_runs = perf.success_count + perf.failure_count;
        perf.average_latency_ms =
            (perf.average_latency_ms * (total_runs - 1) + latency_ms) / total_runs;

        // Check for culling
        if total_runs > 10 {
            let failure_rate = perf.failure_count as f64 / total_runs as f64;
            if failure_rate > self.culling_threshold {
                warn!("🧹 [SkillArena] CULLING DETECTED: Skill '{}' has {}% failure rate. Marking for decommissioning.", skill_name, failure_rate * 100.0);
                // In a real scenario, this might trigger a deletion or a move to 'Untrusted' quarantine.
            }
        }
    }

    pub async fn get_stats(&self, skill_name: &str) -> Option<SkillPerformance> {
        let map = self.performance_map.read().await;
        map.get(skill_name).cloned()
    }
}
