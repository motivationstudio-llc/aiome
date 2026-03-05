/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Bastion - Security Toolkit
//!
//! プロジェクトのセキュリティを総合的にチェック・強化するツールキット。
//! 
//! ## Security Module v2.0 (Industrial Grade)
//! 
//! - `fs_guard`: File Jail (パス・トラバーサル / TOCTOU 防止)
//! - `net_guard`: Net Shield (SSRF / DNS Rebinding 防止)
//! - `text_guard`: Analyzer & Sanitizer (DoS / Bidi / インジェクション検知・防止)

pub mod common;
pub mod guardrails;
pub mod init;
pub mod python_check;
pub mod scanner;

// v2.0 Security Modules
#[cfg(feature = "fs")]
pub mod fs_guard;

#[cfg(feature = "net")]
pub mod net_guard;

#[cfg(feature = "text")]
pub mod text_guard;
