/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # Core — ドメインロジック層
//!
//! Framework のビジネスロジックを定義する。
//! 具体的なI/O実装は `infrastructure` クレートに委譲する（依存性逆転의原則）。

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod biome;
pub mod budget;
pub mod commerce;
pub mod contracts;
pub mod error;
pub mod expression;
pub mod llm_provider;
pub mod traits;
