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

#![allow(warnings)]

pub mod aiome_log;
pub mod artifact_store;
pub mod channel_bridge;
pub mod concept_manager;
pub mod context_engine;
pub mod dream_state;
pub mod heartbeat_wakeup;
pub mod immune_system;
pub mod job_queue;
pub mod knowledge_indexer;
pub mod llm;
pub mod memory_crystallizer;
pub mod oracle;
pub mod publisher;
pub mod security;
pub mod skill_arena;
pub mod skills;
pub mod soul_mutator;
pub mod trend_sonar;
pub mod user_learner;
pub mod validator;
pub mod workspace_manager;
mod workspace_manager_tests;
