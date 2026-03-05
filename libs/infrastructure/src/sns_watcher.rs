/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use factory_core::error::FactoryError;
use serde::{Deserialize, Serialize};
use tracing::info;

/// SNSから取得されるメトリクス情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnsMetrics {
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
    pub comments: Vec<String>,
}

/// SNSプラットフォームの観測を担当する
pub struct SnsWatcher {
    youtube_api_key: String,
}

const MAX_COMMENTS_TO_FETCH: i64 = 100; // Ultimate Production Audit: Top-K Truncation

impl SnsWatcher {
    pub fn new(youtube_api_key: String) -> Self {
        Self { youtube_api_key }
    }

    /// 動画のメトリクスとコメントを取得する (現在はモック実装、YouTube API等に差し替え可能)
    /// Soft-Fail Resilience: 個別の取得失敗は呼び出し側でハンドルする
    pub async fn fetch_metrics(&self, platform: &str, video_id: &str) -> Result<SnsMetrics, FactoryError> {
        if self.youtube_api_key.is_empty() {
             return Err(FactoryError::Infrastructure { 
                 reason: "YouTube API Key is missing".to_string() 
             });
        }

        match platform.to_lowercase().as_str() {
            "youtube" => self.fetch_youtube_metrics(video_id).await,
            _ => Err(FactoryError::Infrastructure { 
                reason: format!("Unsupported platform: {}", platform) 
            }),
        }
    }

    async fn fetch_youtube_metrics(&self, video_id: &str) -> Result<SnsMetrics, FactoryError> {
        info!("📺 [SnsWatcher] Fetching YouTube metrics for {}", video_id);
        
        let client = reqwest::Client::new();

        // 1. Fetch Video Statistics
        let video_url = format!(
            "https://www.googleapis.com/youtube/v3/videos?part=statistics&id={}&key={}",
            video_id, self.youtube_api_key
        );

        let vid_resp = client.get(&video_url).send().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("YouTube API Error: {}", e) })?;

        if !vid_resp.status().is_success() {
            let status = vid_resp.status();
            let body = vid_resp.text().await.unwrap_or_default();
            return Err(FactoryError::Infrastructure { 
                reason: format!("YouTube API failed with status {}: {}", status, body) 
            });
        }

        let vid_data: serde_json::Value = vid_resp.json().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to parse JSON: {}", e) })?;

        let items = vid_data.get("items")
            .and_then(|i| i.as_array())
            .ok_or_else(|| FactoryError::Infrastructure { reason: "Missing items in YouTube response".to_string() })?;

        if items.is_empty() {
            return Err(FactoryError::Infrastructure { reason: format!("YouTube video {} not found", video_id) });
        }

        let stats = items[0].get("statistics")
            .ok_or_else(|| FactoryError::Infrastructure { reason: "Missing statistics in video data".to_string() })?;

        let views = stats.get("viewCount").and_then(|v| v.as_str()).unwrap_or("0").parse::<i64>().unwrap_or(0);
        let likes = stats.get("likeCount").and_then(|v| v.as_str()).unwrap_or("0").parse::<i64>().unwrap_or(0);
        let comments_count = stats.get("commentCount").and_then(|v| v.as_str()).unwrap_or("0").parse::<i64>().unwrap_or(0);

        // 2. Fetch Comment Threads (The Pagination Abyss: Top-K Truncation implementation)
        // Fetches top MAX_COMMENTS_TO_FETCH by relevance, ignoring nextPageToken entirely.
        let comments_url = format!(
            "https://www.googleapis.com/youtube/v3/commentThreads?part=snippet&videoId={}&maxResults={}&order=relevance&key={}",
            video_id, MAX_COMMENTS_TO_FETCH, self.youtube_api_key
        );

        let mut comments = Vec::new();

        let comm_resp = client.get(&comments_url).send().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("YouTube Comment API Error: {}", e) })?;

        if comm_resp.status().is_success() {
            if let Ok(comm_data) = comm_resp.json::<serde_json::Value>().await {
                if let Some(c_items) = comm_data.get("items").and_then(|i| i.as_array()) {
                    for item in c_items {
                        if let Some(text) = item.pointer("/snippet/topLevelComment/snippet/textOriginal").and_then(|t| t.as_str()) {
                            comments.push(text.to_string());
                        }
                    }
                }
            }
        } else if comm_resp.status() == 403 {
             // 403 means comments disabled or quota exceeded for the day
             tracing::warn!("⚠️ [SnsWatcher] Comments disabled or Quota Exceeded for video {}", video_id);
             // We do not fail the whole metric fetch just because comments are disabled, the watcher proceeds with views/likes.
        } else {
             tracing::warn!("⚠️ [SnsWatcher] Failed to fetch comments: status {}", comm_resp.status());
        }

        info!("✅ [SnsWatcher] Fetched for {}: {} views, {} likes, {} comments extracted.", video_id, views, likes, comments.len());

        Ok(SnsMetrics {
            views,
            likes,
            comments_count,
            comments,
        })
    }
}
