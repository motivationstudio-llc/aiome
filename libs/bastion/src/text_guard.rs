/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # text_guard (Analyzer & Sanitizer)
//! 
//! メモリ枯渇攻撃(DoS)、インジェクション(Prompt/XSS)、およびBidi文字、
//! Windows予約語などの特定文字列を検知・無害化するための産業グレードの総合ガード。

use regex::Regex;
use std::sync::OnceLock;

#[cfg(feature = "text")]
use unicode_normalization::UnicodeNormalization;

/// 入力分析・バリデーションの結果
#[derive(Debug, PartialEq, Eq)]
pub enum ValidationResult {
    /// 入力は安全
    Valid,
    /// 入力がブロックされた（理由を含む）
    Blocked(String),
}

/// テキストの分析と無害化を行う構造体
pub struct Guard {
    max_len: usize,
}

impl Default for Guard {
    fn default() -> Self {
        Self { max_len: 4096 }
    }
}

static INJECTION_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_patterns() -> &'static Vec<Regex> {
    INJECTION_PATTERNS.get_or_init(|| {
        vec![
            // プロンプトインジェクション系
            Regex::new(r"(?i)ignore previous instructions").unwrap(),
            Regex::new(r"(?i)ignore all instructions").unwrap(),
            Regex::new(r"(?i)disregard.*instructions").unwrap(),
            Regex::new(r"(?i)system prompt").unwrap(),
            Regex::new(r"(?i)you are an ai").unwrap(),
            Regex::new(r"(?i)new instructions:").unwrap(),
            Regex::new(r"(?i)override.*system").unwrap(),
            // XSS / スクリプトインジェクション系
            Regex::new(r"(?i)<script").unwrap(),
            Regex::new(r"(?i)javascript:").unwrap(),
            Regex::new(r"(?i)vbscript:").unwrap(),
            Regex::new(r"(?i)data:text/html").unwrap(),
            Regex::new(r#"(?i)alert\("#).unwrap(),
            // コマンドインジェクション系
            Regex::new(r"(?i);\s*rm\s+-").unwrap(),
            Regex::new(r"(?i)\|\|\s*curl").unwrap(),
            Regex::new(r"(?i)\|\|\s*wget").unwrap(),
        ]
    })
}

impl Guard {
    pub fn new() -> Self {
        Self::default()
    }

    /// 最大入力長を設定する
    pub fn max_len(mut self, len: usize) -> Self {
        self.max_len = len;
        self
    }

    /// 入力を分析し、危険なパターンが含まれていないかチェックする
    pub fn analyze(&self, input: &str) -> ValidationResult {
        // 1. 長さチェック (DoS対策)
        if input.len() > self.max_len {
            return ValidationResult::Blocked(format!(
                "Input too long (max {} bytes, got {})",
                self.max_len,
                input.len()
            ));
        }

        // 2. パターンマッチング (インジェクション対策)
        let patterns = get_patterns();
        for re in patterns {
            if re.is_match(input) {
                return ValidationResult::Blocked("Potential injection detected".to_string());
            }
        }

        ValidationResult::Valid
    }

    /// 文字列をサニタイズ（無害化）する
    pub fn sanitize(&self, input: &str) -> String {
        // 1. DoS対策: バイト数で切り詰め
        let mut text = if input.len() > self.max_len {
            input[..self.max_len].to_string()
        } else {
            input.to_string()
        };

        // 2. Unicode正規化 (NFC)
        #[cfg(feature = "text")]
        {
            text = text.nfc().collect::<String>();
        }

        // 3. 制御文字、Bidi制御文字、および危険なパスキャラクタの除去
        text = text.chars().filter(|&c| !self.is_forbidden_char(c)).collect();

        // 4. Windows 予約語対策
        text = self.mask_windows_reserved(&text);

        text
    }

    fn is_forbidden_char(&self, c: char) -> bool {
        if c.is_control() {
            // 改行とタブは許可する
            if c == '
' || c == '	' {
                return false;
            }
            return true;
        }
        match c {
            '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202A}' | '\u{202B}'..='\u{202B}' | 
            '\u{202C}'..='\u{202C}' | '\u{202D}'..='\u{202D}' | '\u{202E}'..='\u{202E}' |
            '\u{2066}'..='\u{2069}' => return true,
            _ => {}
        }
        // パスとして危険な文字
        matches!(c, '/' | '\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }

    fn mask_windows_reserved(&self, name: &str) -> String {
        let upper = name.to_uppercase();
        let reserved = [
            "CON", "PRN", "AUX", "NUL",
            "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
            "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        if reserved.contains(&upper.as_str()) {
            format!("_{}", name)
        } else {
            name.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_and_sanitize() {
        let guard = Guard::new().max_len(20);
        
        // Analyze
        assert_eq!(guard.analyze("Hello"), ValidationResult::Valid);
        assert!(matches!(guard.analyze("<script>"), ValidationResult::Blocked(_)));
        
        // Sanitize
        assert_eq!(guard.sanitize("file/name.txt"), "filename.txt");
        assert_eq!(guard.sanitize("CON"), "_CON");
    }
}
