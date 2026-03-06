/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # OutputValidator — LLM 出力のバリデーション
//!
//! Guardrails が「入力側」を守るのに対し、OutputValidator は「出力側」を守る。
//! LLM が返す JSON が Rust の型定義に適合しない場合に、
//! エラー情報を LLM にフィードバックして自己修復を試みる。

use serde::de::DeserializeOwned;

/// LLM 出力のバリデーション結果
#[derive(Debug)]
pub enum ValidationResult<T> {
    /// パース成功
    Valid(T),
    /// パース失敗 — 修正用のフィードバックメッセージを含む
    Invalid {
        raw_output: String,
        error_message: String,
        repair_prompt: String,
    },
}

/// LLM の JSON 出力を型安全にパースし、失敗時は修正プロンプトを生成する
///
/// # Self-Healing Parse (自己修復パース)
///
/// 1. LLM の出力文字列から JSON 部分を抽出
/// 2. 指定された Rust 型へのデシリアライズを試みる
/// 3. 失敗した場合、エラー内容を含む「修正指示プロンプト」を生成
///
/// 呼び出し側はこの修正プロンプトを LLM に再送し、リトライできる。
pub fn validate_json_output<T: DeserializeOwned>(
    raw_output: &str,
) -> ValidationResult<T> {
    // Step 1: JSON ブロックを抽出（```json ... ``` や 生の JSON に対応）
    let json_str = extract_json_block(raw_output);

    // Step 2: デシリアライズを試みる
    match serde_json::from_str::<T>(&json_str) {
        Ok(parsed) => ValidationResult::Valid(parsed),
        Err(e) => {
            let error_msg = format!("{}", e);
            let repair_prompt = build_repair_prompt(&json_str, &error_msg);
            ValidationResult::Invalid {
                raw_output: raw_output.to_string(),
                error_message: error_msg,
                repair_prompt,
            }
        }
    }
}

/// LLM 出力から JSON ブロックを抽出する
///
/// 以下のパターンに対応:
/// 1. ```json ... ``` で囲まれた JSON
/// 2. { ... } で始まる生の JSON
/// 3. [ ... ] で始まる配列 JSON
fn extract_json_block(raw: &str) -> String {
    // Pattern 1: ```json ... ```
    if let Some(start) = raw.find("```json") {
        let content_start = start + 7;
        if let Some(end) = raw[content_start..].find("```") {
            return raw[content_start..content_start + end].trim().to_string();
        }
    }

    // Pattern 2: ``` ... ``` (no language specifier)
    if let Some(start) = raw.find("```") {
        let content_start = start + 3;
        if let Some(end) = raw[content_start..].find("```") {
            let block = raw[content_start..content_start + end].trim();
            if block.starts_with('{') || block.starts_with('[') {
                return block.to_string();
            }
        }
    }

    // Pattern 3: 最初の { ... } を探す
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            if end > start {
                return raw[start..=end].to_string();
            }
        }
    }

    // Pattern 4: 最初の [ ... ] を探す
    if let Some(start) = raw.find('[') {
        if let Some(end) = raw.rfind(']') {
            if end > start {
                return raw[start..=end].to_string();
            }
        }
    }

    // 見つからない場合は元の文字列をそのまま返す
    raw.trim().to_string()
}

/// パースエラーから修正指示プロンプトを生成
fn build_repair_prompt(invalid_json: &str, error: &str) -> String {
    // Secondary Prompt Injection 対策:
    // 注入されたバックティックによってプロンプトの構造が破壊されるのを防ぐため、
    // バックティックをエスケープまたは置換する。
    let safe_json = invalid_json.replace("```", "'''");
    
    format!(
        "あなたの前回の出力は JSON パースに失敗しました。以下の情報を元に、正しい JSON を再生成してください。

## エラー内容
{}

## あなたの前回の出力（問題あり）
```json
{}
```

## ルール
- 必ず有効な JSON **のみ** を出力してください（説明文は不要）。
- 数値フィールドに文字列を入れないでください。
- 必須フィールドを省略しないでください。
- 配列が期待される場所にはオブジェクトを入れないでください。",
        error, safe_json
    )
}

/// 最大リトライ回数のデフォルト値
pub const DEFAULT_MAX_RETRIES: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        count: u32,
    }

    #[test]
    fn test_valid_json_parses() {
        let input = r#"{"name": "test", "count": 42}"#;
        match validate_json_output::<TestStruct>(input) {
            ValidationResult::Valid(v) => {
                assert_eq!(v.name, "test");
                assert_eq!(v.count, 42);
            }
            ValidationResult::Invalid { .. } => panic!("Expected Valid"),
        }
    }

    #[test]
    fn test_extracts_json_from_markdown() {
        let input = "Here is the result:
```json
{\"name\": \"hello\", \"count\": 10}
```
Done!";
        match validate_json_output::<TestStruct>(input) {
            ValidationResult::Valid(v) => {
                assert_eq!(v.name, "hello");
                assert_eq!(v.count, 10);
            }
            ValidationResult::Invalid { .. } => panic!("Expected Valid"),
        }
    }

    #[test]
    fn test_invalid_json_returns_repair_prompt() {
        let input = r#"{"name": "test", "count": "not_a_number"}"#;
        match validate_json_output::<TestStruct>(input) {
            ValidationResult::Invalid {
                repair_prompt,
                error_message,
                ..
            } => {
                assert!(!repair_prompt.is_empty());
                assert!(!error_message.is_empty());
                assert!(repair_prompt.contains("再生成"));
            }
            ValidationResult::Valid(_) => panic!("Expected Invalid"),
        }
    }

    #[test]
    fn test_missing_field_returns_repair_prompt() {
        let input = r#"{"name": "test"}"#;
        match validate_json_output::<TestStruct>(input) {
            ValidationResult::Invalid { error_message, .. } => {
                assert!(error_message.contains("count"));
            }
            ValidationResult::Valid(_) => panic!("Expected Invalid"),
        }
    }

    #[test]
    fn test_extracts_json_from_prose() {
        let input = "The answer is {\"name\": \"embedded\", \"count\": 5} and that's it.";
        match validate_json_output::<TestStruct>(input) {
            ValidationResult::Valid(v) => {
                assert_eq!(v.name, "embedded");
                assert_eq!(v.count, 5);
            }
            ValidationResult::Invalid { .. } => panic!("Expected Valid"),
        }
    }

    #[test]
    fn test_completely_invalid_input() {
        let input = "This is just plain text with no JSON";
        let result = validate_json_output::<TestStruct>(input);
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }
}
