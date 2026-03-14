/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod aiome_log;
pub mod artifact_store;
pub mod channel_bridge;
pub mod circuit_breaker;
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
pub mod slo_engine;
pub mod soul_mutator;
pub mod trend_sonar;
pub mod user_learner;
pub mod validator;
pub mod workspace_manager;
mod workspace_manager_tests;
