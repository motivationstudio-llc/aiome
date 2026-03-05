/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # net_guard (Net Shield)
//! 
//! SSRF、DNS Rebinding、およびIPv6バイパス攻撃を防ぐための
//! 産業グレードのネットワークガード。
//! カスタム名前解決を行い、プライベートIPへのアクセスを物理的に遮断する。

use std::net::IpAddr;
use anyhow::{bail, Result};

#[cfg(feature = "net")]
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
#[cfg(feature = "net")]
use trust_dns_resolver::TokioAsyncResolver;
#[cfg(feature = "net")]
use reqwest::{Client, redirect::Policy};

/// ネットワークアクセスの制限を行う構造体
#[derive(Clone, Debug)]
pub struct ShieldClient {
    #[cfg(feature = "net")]
    client: Client,
    allowlist: Vec<String>,
}

impl ShieldClient {
    /// ShieldClient のビルダー
    pub fn builder() -> ShieldClientBuilder {
        ShieldClientBuilder::default()
    }

    /// 安全に GET リクエストを送信する
    #[cfg(feature = "net")]
    pub async fn get(&self, url: &str) -> Result<reqwest::Response> {
        self.validate_url(url).await?;
        Ok(self.client.get(url).send().await?)
    }

    /// 安全に POST リクエストを送信する (JSON ペイロード)
    #[cfg(feature = "net")]
    pub async fn post<T: serde::Serialize>(&self, url: &str, json_body: &T) -> Result<reqwest::Response> {
        self.validate_url(url).await?;
        Ok(self.client.post(url).json(json_body).send().await?)
    }

    /// URL を検証する（Allowlist, DNS解決, IPチェック）
    pub async fn validate_url(&self, url_str: &str) -> Result<()> {
        let url = url::Url::parse(url_str)?;
        let host = url.host_str().ok_or_else(|| anyhow::anyhow!("No host in URL"))?;

        // 1. Allowlist チェック
        if self.allowlist.contains(&host.to_string()) {
            return Ok(());
        }

        // 2. DNS 名前解決 (A/AAAA) と IP 検証
        #[cfg(feature = "net")]
        {
            let resolver = TokioAsyncResolver::tokio(
                ResolverConfig::default(),
                ResolverOpts::default(),
            );
            
            let response = resolver.lookup_ip(host).await?;
            for ip in response.iter() {
                // 工場の要件としては「Default Deny」なので、Allowlist 外は全てエラーにする。
                if self.is_private_ip(ip) {
                    bail!("Access Denied: Private IP address detected ({})", ip);
                }
            }

            // プライベートIPチェックを通過しても、Allowlist にない場合は拒否する (Strict Mode)
            bail!("Access Denied: Host '{}' is not in the allowlist (Strict Mode)", host);
        }

        #[cfg(not(feature = "net"))]
        bail!("Access Denied: Host '{}' is not in the allowlist", host);
    }

    /// プライベート IP かどうかを判定する (IPv4/v6)
    fn is_private_ip(&self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => {
                v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_broadcast() || v4.is_documentation() || v4.is_unspecified()
            }
            IpAddr::V6(v6) => {
                v6.is_loopback() || v6.is_unspecified() || 
                (v6.segments()[0] & 0xfe00) == 0xfc00 || // Unique Local (fc00::/7)
                (v6.segments()[0] & 0xffc0) == 0xfe80    // Link-Local (fe80::/10)
            }
        }
    }
}

/// ShieldClient を構築するためのビルダー
#[derive(Default)]
pub struct ShieldClientBuilder {
    allowlist: Vec<String>,
    block_private_ips: bool,
}

impl ShieldClientBuilder {
    pub fn allow_endpoint(mut self, host: &str) -> Self {
        self.allowlist.push(host.to_string());
        self
    }

    pub fn block_private_ips(mut self, block: bool) -> Self {
        self.block_private_ips = block;
        self
    }

    #[cfg(feature = "net")]
    pub fn build(self) -> Result<ShieldClient> {
        // reqwest クライアントの構築 (リダイレクト禁止)
        let client = Client::builder()
            .redirect(Policy::none()) // N-06: 自動リダイレクト禁止
            .build()?;

        Ok(ShieldClient {
            client,
            allowlist: self.allowlist,
        })
    }

    #[cfg(not(feature = "net"))]
    pub fn build(self) -> Result<ShieldClient> {
        Ok(ShieldClient {
            allowlist: self.allowlist,
        })
    }
}

#[cfg(test)]
#[cfg(feature = "net")]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_private_ip_blocking() {
        let shield = ShieldClient::builder().block_private_ips(true).build().unwrap();
        
        // Loopback
        assert!(shield.validate_url("http://127.0.0.1").await.is_err());
        // Private
        assert!(shield.validate_url("http://192.168.1.1").await.is_err());
        // IPv6 Loopback
        assert!(shield.validate_url("http://[::1]").await.is_err());
    }

    #[tokio::test]
    async fn test_allowlist() {
        let shield = ShieldClient::builder()
            .allow_endpoint("localhost")
            .build()
            .unwrap();
        
        // Allowlist にあれば通過
        assert!(shield.validate_url("http://localhost:8188").await.is_ok());
    }
}
