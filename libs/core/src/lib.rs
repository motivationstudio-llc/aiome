/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

//! # Core — ドメインロジック層
//!
//! Framework のビジネスロジックを定義する。
//! 具体的なI/O実装は `infrastructure` クレートに委譲する（依存性逆転の原則）。

pub mod error;
pub mod traits;
pub mod contracts;
pub mod budget;
pub mod llm_provider;
