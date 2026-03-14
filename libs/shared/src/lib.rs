/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cleaner;
pub mod config;

pub mod guardrails;
pub mod health;
pub mod os_utils;
pub mod output_validator;
pub mod sandbox;
pub mod security;
pub mod watchtower;
pub mod zombie_killer;
