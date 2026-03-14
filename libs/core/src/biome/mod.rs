/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

pub mod autonomous;
pub mod delegation;
pub mod dialogue;
pub mod protocol;

pub use autonomous::{AutonomousBiomeEngine, AutonomousConfig};
pub use delegation::{DelegationResult, FailureCategory};
pub use protocol::{BiomeDialogue, BiomeMessage, DialogueStatus};
