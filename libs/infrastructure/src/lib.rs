/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
//! ComfyUI, FFmpeg, SQLite 等の外部サービスとの通信を担当。

pub mod comfy_bridge;
pub mod concept_manager;
pub mod factory_log;
pub mod media_forge;
pub mod trend_sonar;
pub mod voice_actor;
pub mod sound_mixer;
pub mod job_queue;
mod job_queue_tests;
pub mod workspace_manager;
mod workspace_manager_tests;
pub mod sns_watcher;
pub mod oracle;
pub mod skills;
pub mod soul_mutator;
pub mod dream_state;
pub mod immune_system;
pub mod skill_arena;
