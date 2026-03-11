/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use serde::{Deserialize, Serialize};
use base64::Engine;

/// Biome プロトコルにおける基本メッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMessage {
    /// 送信元の公開鍵 (Base64)
    pub sender_pubkey: String,
    /// 宛先の公開鍵 (Base64)
    pub recipient_pubkey: String,
    /// 対話のトピックID (Dialogue ID)
    pub topic_id: String,
    /// メッセージ本文 (暗号化されている場合は暗号文)
    pub content: String,
    /// 送信者の Karma 状態を参照する CID (Merkle DAG)
    pub karma_root_cid: String,
    /// Ed25519 署名
    pub signature: String,
    /// Lamport Clock 等のロジカル時刻
    pub lamport_clock: u64,
    /// タイムスタンプ
    pub timestamp: String,
    /// 暗号化方式 ("none", "aes-256-gcm", etc.)
    /// Phase 20 MVP では "none" を使用。
    pub encryption: String,
}

/// 対話の要約・状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDialogue {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub last_message_at: String,
    pub message_count: u32,
    pub summary: Option<String>,
    pub status: DialogueStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DialogueStatus {
    Active,
    Archived,
    Blocked,
}

impl BiomeMessage {
    /// メッセージ本文を指定された共有鍵で暗号化する
    pub fn encrypt(&mut self, key: &[u8; 32]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};
        
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(b"biome-proto-v1"); // MVP fixed nonce - in production should be random/lamport BASED
        
        let ciphertext = cipher.encrypt(nonce, self.content.as_bytes())
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        
        self.content = base64::engine::general_purpose::STANDARD.encode(ciphertext);
        self.encryption = "chacha20-poly1305".to_string();
        Ok(())
    }

    /// メッセージ本文を指定された共有鍵で復号する
    pub fn decrypt(&mut self, key: &[u8; 32]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.encryption == "none" {
            return Ok(());
        }
        
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};
        use base64::Engine;

        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(b"biome-proto-v1");
        
        let ciphertext = base64::engine::general_purpose::STANDARD.decode(&self.content)?;
        let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
            .map_err(|e| format!("Decryption failed: {:?}", e))?;
        
        self.content = String::from_utf8(plaintext)?;
        Ok(())
    }
}
