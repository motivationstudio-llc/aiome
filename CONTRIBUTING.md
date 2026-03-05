# Contributing to Aiome
## Aiome への貢献 (日本語併記)

First of all, thank you for considering contributing! It's people like you that make Aiome better for everyone.
まず最初に、Aiomeへの貢献をご検討いただきありがとうございます！皆さんのような協力者が、Aiomeをより良いものにしていきます。

---

## 🏗️ Our Open-Core Model (オープンコア・モデル)

Aiome follows an **Open-Core** model:
Aiomeは**オープンコア**モデルを採用しています：
- **Aiome Core (OSS)**: Under AGPL-3.0. Includes the framework, Karma system, and basic Watchtower features. (基本フレームワーク、Karmaシステム、および基本的なWatchtower機能を含むOSS版)
- **Aiome Pro/Enterprise**: Features for mass-scale GPU orchestration and advanced Skill Forge capabilities are proprietary. (大規模なGPUオーケストレーションや高度なSkill Forge機能を含む商用版)

---

## 📝 How Can I Contribute? (貢献方法)

- **Bug Reports**: Open an issue with a clear description and steps to reproduce. (バグ報告: 再現手順を含む明確な説明とともにIssueを作成してください)
- **Feature Requests**: We love ideas! Please check if a similar request already exists. (機能提案: 新しいアイデアは歓迎です！既存の提案と重複していないか確認してください)
- **Code Contribution**: Fork the repo, create a branch, and submit a Pull Request. (コード貢献: リポジトリをフォークし、ブランチを作成してPRを提出してください)

---

## 🛠️ Development Setup (開発セットアップ)

The project is built with **Rust**.

### Prerequisites (前提条件)
- **Rust**: 1.75+ (Stable)
- **Ollama**: For local LLM processing (Qwen2.5-Coder recommended).
- **ComfyUI**: Required for image/video generation tasks.
- **FFmpeg**: Required for media processing.

### Building & Testing (ビルドとテスト)
```bash
# Build (ビルド)
cargo build --workspace

# Test (テスト)
cargo test --workspace
```

---

## 🚀 Pull Request Process (PRプロセス)

1. Fork the repository and create your branch from `main`. (forkしてmainからブランチ作成)
2. If you've added code that should be tested, add tests. (必要に応じてテストを追加)
3. Ensure the test suite passes. (テストが通ることを確認)
4. Run license compliance check: `cargo deny check license`. (ライセンステストの実行)
5. **Sign the CLA**: Your PR will only be merged once the CLA check passes. (CLAへの同意)
6. Submit the PR! (提出！)

---

## 🏛️ Coding Standards & Architecture (設計原則)

We follow a strict **Modular Workspace** architecture. (厳格なモジュール型ワークスペース構成を採用しています)

- **apps/api-server & shorts-factory**: Main entry points and DI containers. (エントリーポイントとDIコンテナ)
- **libs/infrastructure**: Handle I/O (DB, Redis, External APIs). (I/O実装)
- **libs/core**: Pure Domain Logic, Entities, and Interfaces. (純粋なドメインロジック)
    - **CRITICAL**: `core` MUST NOT depend on `infrastructure` (Dependency Inversion Principle). (`core`が`infrastructure`に依存してはいけません - 依存性逆転の原則)
- **libs/shared**: Common utils and types. MUST NOT depend on any other layers. (共通定義。他レイヤに依存してはいけません)

#### Iron Principles (鉄の掟):
- **Result Type Mandatory**: No `unwrap()` or `expect()` outside of tests. (`unwrap()` などの禁止)
- **Type Safety**: Use NewType patterns and Enums for data flow. (型安全性の徹底)
- **Async First**: All I/O must be non-blocking using `tokio`. (非同期処理の徹底)
- **Error Handling**: Use `anyhow` for apps and `thiserror` for library layers. (エラー処理の使い分け)

---

## ⚖️ License & CLA (ライセンスとCLA)

By contributing, you agree that your contributions will be licensed under **AGPL-3.0** and you agree to the terms of our **[CLA.md](./CLA.md)**.
貢献を行うことにより、あなたのコードが **AGPL-3.0** としてライセンスされること、および **[CLA.md](./CLA.md)** の条項に同意したとみなされます。

---
*Happy coding!*
