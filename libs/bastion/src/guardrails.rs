/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Guardrails - 入力バリデーションモジュール
//!
//! `text_guard` モジュールを利用した簡易バリデーションを提供する。
//! より高度な制御が必要な場合は `bastion::text_guard::Guard` を直接使用してください。

use crate::text_guard::{Guard, ValidationResult};

/// デフォルト設定で入力を検証する
pub fn validate_input(input: &str) -> ValidationResult {
    Guard::new().analyze(input)
}

/// 最大長を指定して入力を検証する
pub fn validate_input_with_max_len(input: &str, max_len: usize) -> ValidationResult {
    Guard::new().max_len(max_len).analyze(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        assert_eq!(validate_input("Safe input"), ValidationResult::Valid);
        assert!(matches!(validate_input("<script>"), ValidationResult::Blocked(_)));
    }
}
