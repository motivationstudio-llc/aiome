/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

/// Biome プロトコルの対話ルール
/// - State Channel: 1件のトピックにつき最大10往復
/// - 往復終了後に LLM による要約を行い、アーカイブ化
pub const MAX_DIALOGUE_TURNS: u32 = 10;
