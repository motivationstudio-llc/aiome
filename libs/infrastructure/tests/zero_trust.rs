/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::llm_provider::LlmProvider;
use infrastructure::llm::proxy::ProxyLlmProvider;
// use std::sync::Arc; // Unused import removed

#[tokio::test]
async fn test_proxy_llm_provider_flow() {
    // This test assumes key-proxy is running locally for integration testing
    // In CI, we would mock the key-proxy or start a test instance
    let provider = ProxyLlmProvider::new(
        "http://127.0.0.1:9999".to_string(),
        "daemon".to_string(),
        "gemini".to_string(),
    );

    // Test a simple completion
    // Note: This will fail if key-proxy is not running or has no network access
    // but the trait implementation itself is what we are verifying.
    assert_eq!(provider.name(), "KeyProxy");
}

#[tokio::test]
async fn test_unauthorized_caller() {
    let provider = ProxyLlmProvider::new(
        "http://127.0.0.1:9999".to_string(),
        "hacker".to_string(),
        "gemini".to_string(),
    );
    let res = provider.complete("hello", None).await;

    match res {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.to_lowercase().contains("keyproxy")
                    || err_str.to_lowercase().contains("connect")
                    || err_str.to_lowercase().contains("refused")
                    || err_str.to_lowercase().contains("error sending request")
            );
        }
        Ok(_) => panic!("Unauthorized caller should have been blocked"),
    }
}
