/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
//! 外部サービスとの通信やデータ永続化を担当する。

pub mod concept_manager;
pub mod aiome_log;
pub mod trend_sonar;
pub mod job_queue;
mod job_queue_tests;
pub mod workspace_manager;
mod workspace_manager_tests;
pub mod oracle;
pub mod skills;
pub mod soul_mutator;
pub mod dream_state;
pub mod immune_system;
pub mod skill_arena;
pub mod llm;
pub mod security;
