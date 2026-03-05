/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Core — ドメインロジック層
//!
//! ShortsFactory のビジネスロジックを定義する。
//! 具体的なI/O実装は `infrastructure` クレートに委譲する（依存性逆転の原則）。

pub mod error;
pub mod traits;
pub mod contracts;
pub mod budget;
