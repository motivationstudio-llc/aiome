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
