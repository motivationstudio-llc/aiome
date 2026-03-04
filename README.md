# Aiome (アイオーム) — The Independent AI Evolution Engine

YouTube Shorts / TikTok 向けの動画を**全自動で量産**する、Rust ネイティブの自律型動画工場。

## アーキテクチャ (Open-Core Strategy)

当プロジェクトは**オープンコアモデル**を採用しています。

### 🟢 Aiome Core (OSS版 - AGPL-3.0)
基本フレームワーク、Karma（学習履歴）スキーム、および Watchtower 連携機能はオープンソースとして提供されます。
コミュニティ全体で教訓（Karma）を共有し、AIの「群知能」を進化させることを目的とします。
（※OSS版の利用は、メタデータ化された匿名Karmaをフェデレーションネットワークへ同期することに同意したとみなされます）

### 🔴 Aiome Pro / Enterprise (クローズド版・商用)
高度なジョブ調停（GPU並列稼働など）、動的WASMコンパイラ（Skill Forge）、および商用特化のキラーモジュールは、個別の商用ライセンスとして提供されます。

```text
apps/shorts-factory  ← メインバイナリ (The Body / Open & Pro)
      ↓
libs/core            ← ドメインロジック (Open)
      ↓
libs/infrastructure  ← I/O実装 (ComfyUI, SQLite / Open)
      ↓
libs/shared          ← 共通型, Guardrails (Open)
```

## 技術スタック

| コンポーネント | 技術 |
|---|---|
| 言語 | Rust (メモリ安全・ネイティブ速度) |
| LLM | Qwen 2.5-Coder via Ollama |
| Agent | rig-core v0.30 |
| 画像/動画生成 | ComfyUI (localhost:8188) |
| 動画編集 | FFmpeg |
| データベース | SQLite |

## セキュリティ

3層防御 + CI 自動スキャン:

- **Guardrails**: プロンプトインジェクション検知 (ランタイム)
- **SecurityPolicy**: ツール/ネットワークのホワイトリスト (ランタイム)
- **Sentinel**: シークレットスキャン + `cargo audit` + unsafe 検出 (CI)

詳細: [docs/SECURITY_DESIGN.md](docs/SECURITY_DESIGN.md)

## 実行コンポーネント

### 1. 工場本体 (Aiome Core / Command Center)
```bash
# サーバーモードで起動 (GUI / Discord連携に必須)
cargo run -p shorts-factory -- serve
```
- Web UI: `http://localhost:3000` (コマンドセンター)
- API Port: `5000`

### 2. 監視所 (Watchtower - Discord Bot)
```bash
# 別ターミナルで起動 (.env にトークンが必要)
cargo run -p watchtower
```
- コマンド: `/status`, `/stats`, `/nuke`, `/generate`
- 詳細: [docs/WATCHTOWER_USER_GUIDE.md](docs/WATCHTOWER_USER_GUIDE.md)

### 3. エージェント育成・進化 (Evolution System)
- **Project Ani**: 交流と成功体験による AI の人格成長。
- **Unleashed Mode**: 全ての制限を解除する Platinum Edition フラグ。
- 詳細: [docs/EVOLUTION_STRATEGY.md](docs/EVOLUTION_STRATEGY.md)

## 🚀 クイックスタート (Quick Start)

### 1. 準備
```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
cp .env.example .env  # APIキー等の設定
```

### 2. コンポーネントの起動
- **Ollama**: `ollama serve` & `ollama run qwen2.5-coder:7b` (別ターミナル)
- **ComfyUI**: `python main.py` (別ターミナル)
- **Factory Core**: `cargo run -p shorts-factory -- serve`
- **Watchtower (Discord)**: `cargo run -p watchtower`

---

## 📚 ドキュメント (Documentation)

- **[構成と憲法](docs/CODE_WIKI.md)**: アーキテクチャと Iron Principles。
- **[運用・セットアップ](docs/OPERATIONS_MANUAL.md)**: ハードウェア・ソフトウェア要件と詳細手順。
- **[監視所 (Watchtower)](docs/WATCHTOWER_USER_GUIDE.md)**: Discord連携とコマンドガイド。
- **[セキュリティ設計](docs/SECURITY_DESIGN.md)**: 脅威モデルと多層防御。
- **[進化戦略](docs/EVOLUTION_STRATEGY.md)**: 育成システムと Open-Core 戦略。
- **[人格のカスタマイズ](docs/CUSTOMIZING_SOUL.md)**: `SOUL.md` の設定と運用ガイド。

### 🛡️ コミュニティ & セキュリティ (Community & Security)

- **[セキュリティ報告](SECURITY.md)**: 脆弱性発見時の報告手順。
- **[行動規範](CODE_OF_CONDUCT.md)**: コミュニティの行動基準。
- **[貢献ガイド](CONTRIBUTING.md)**: 開発参加への入り口。

---

## 🤝 コントリビュート (Contributing)

バグ報告、機能提案、コードの寄付を歓迎します。詳細は **[CONTRIBUTING.md](./CONTRIBUTING.md)** をご覧ください。

※PRの提出には **[CLA (Contributor License Agreement)](./CLA.md)** への同意が必要です。

---

## 🛡️ ライセンス (License)

**Aiome Core** は **AGPL-3.0** の下で提供されています。
商用利用や特化機能が必要な場合は、[Aiome Enterprise](https://aiome.dev) をご検討ください。
