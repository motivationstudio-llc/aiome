# Contributing to Aiome
## Aiome への貢献 (日本語併記)

First of all, thank you for considering contributing! It's people like you that make Aiome better for everyone.
まず最初に、Aiomeへの貢献をご検討いただきありがとうございます！皆さんのような協力者が、Aiomeをより良いものにしていきます。

---

## 🏗️ Full Open Source Model (フルオープンソース・モデル)

Aiome follows a **Full Open Source** model:
Aiomeは**完全オープンソース**モデルを採用しています：
- **Aiome Core (OSS)**: Under the Elastic License 2.0 (ELv2). Includes the framework, Karma system, Abyss Vault, and basic Watchtower features. (基本フレームワーク、Karmaシステム、Abyss Vault、および基本的なWatchtower機能を含む完全版)
- **Ecosystem**: Advanced features (like specialized models, premium WASM skills, or managed Samsara Hub operations) are offered as separate modules or services. (高度な特化型WASMスキルや、Samsara HubのSaaS運用などは、独立したモジュールやサービスとして提供されます)

---

## 📝 How Can I Contribute? (貢献方法)

- **Bug Reports**: Open an issue with a clear description and steps to reproduce. (バグ報告)
- **Feature Requests**: We love ideas! Please check if a similar request already exists. (機能提案)
- **Code Contribution**: Fork the repo, create a branch, and submit a Pull Request. (コード貢献)

---

## 🛠️ Development Setup (開発セットアップ)

The project is built with **Rust**.

### Prerequisites (前提条件)
- **Rust**: 1.75+ (Stable)
- **Ollama**: For local LLM processing (`qwen3.5:9b` recommended for background tasks).
- **External Integration**: Access to generative engines or media processing tools is optional and dependent on the skills being developed.

### Building & Testing (ビルドとテスト)
```bash
# Build
cargo build --workspace
# Test
cargo test --workspace
```

---

## 🚀 Pull Request Process (PRプロセス)

1. Fork the repository and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. Ensure the test suite passes.
4. Run license compliance check: `cargo deny check license`.
5. **Sign the CLA**: Your PR will only be merged once the CLA check passes.
6. Submit the PR!

---

## 🏛️ Coding Standards & Architecture (設計原則)

We follow a strict **Modular Workspace** architecture.

- **apps/api-server & aiome-daemon**: Main reference entry points and DI containers.
- **libs/infrastructure**: Handle I/O (DB, Redis, External APIs).
- **libs/core**: Pure Domain Logic, Entities, and Interfaces.
    - **CRITICAL**: `core` MUST NOT depend on `infrastructure` (Dependency Inversion Principle).
- **libs/shared**: Common utils and types. MUST NOT depend on any other layers.

#### Iron Principles (鉄の掟):
- **Result Type Mandatory**: No `unwrap()` or `expect()` outside of tests.
- **Type Safety**: Use NewType patterns and Enums for data flow.
- **Async First**: All I/O must be non-blocking using `tokio`.
- **Error Handling**: Use `anyhow` for apps and `thiserror` for library layers.

---

## ⚖️ License & CLA (ライセンスとCLA)

By contributing, you agree that your contributions will be licensed under the **Elastic License 2.0 (ELv2)** and you agree to the terms of our **[CLA.md](./CLA.md)**.

---
*Happy coding!*
