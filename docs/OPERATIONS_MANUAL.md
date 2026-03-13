# Aiome Operations Manual — 実用運用ガイド
**Version:** 2.0  
**Last Updated:** 2026-03-13

---

## 1. Prerequisites (前提条件)

### 1.1 Hardware
- **推奨**: Mac mini M4 Pro (24GB RAM) 以上
- **Storage**: SSD 10GB+ (データ蓄積のため)

### 1.2 Software Dependencies

| Software | Version | Purpose | Install |
|----------|---------|---------|---------|
| Rust | 1.85+ | コア開発 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | Management Console UI | `brew install node` |
| Python | 3.10+ | ruri-v3 embedding server | `brew install python` |
| Ollama | Latest | バックグラウンドLLM | `brew install ollama` |
| SQLite | 3.40+ | DB (ビルトイン) | Rust `sqlx` に含まれる |

### 1.3 API Keys

| Key | 取得先 | 用途 |
|-----|--------|------|
| `GEMINI_API_KEY` | [Google AI Studio](https://aistudio.google.com/) | フロントエンド推論 (Gemini 2.5 Flash) |

### 1.4 LLM構成 (Pattern B: 推奨)

| 用途 | プロバイダー | モデル | コスト |
|------|-----------|--------|--------|
| フロントエンド (Agent Console) | Gemini Cloud | `gemini-2.5-flash` | 月≈100円 |
| バックグラウンド (Soul Mutator等) | Ollama (Local) | `qwen3.5:9b` | 無料 |

---

## 2. Initial Setup (初期セットアップ)

### 2.1 環境変数の設定

```bash
# プロジェクトルートの .env ファイルを編集
cp .env.example .env

# .env の主要設定:
GEMINI_API_KEY=your_gemini_key_here
BG_LLM_PROVIDER=ollama
BG_LLM_MODEL=qwen3.5:9b
API_SERVER_SECRET=your_random_secret_here
VAULT_SECRET=your_vault_secret
FEDERATION_SECRET=your_hub_secret
```

### 2.2 ビルドと初期検証

```bash
# 1. ビルド
cargo build -p api-server

# 2. テスト実行
cargo test --workspace
```

---

## 3. Commands (コマンド一覧)

### 3.1 API Server 起動

```bash
RUST_LOG=info cargo run -p api-server
# → http://localhost:3015 でManagement Consoleにアクセス
```

### 3.2 Management Console (フロントエンド) 起動

```bash
cd apps/management-console
npm install
npm run dev
# → http://localhost:1420 でアクセス
```

---

## 4. Configuration (設定)

### 4.1 `styles.toml` (演出スタイル定義)
必要に応じて生成スキルのパラメータを定義します。

### 4.2 `SOUL.md` (AI人格定義)
AIの性格や話し方を定義するファイルです。オンボーディング時に設定されます。

### 4.3 Settings UI
`http://localhost:3015` → Settings ページから、以下を変更可能:
- AI Name（AIの表示名）
- Avatar（性別・スタイル）
- LLM Provider（フロントエンド / バックグラウンド）
- Background LLM（プロバイダー / モデル / APIキー）

---

## 5. Database (データベース)

### 5.1 スキーマ概要
- `jobs`: 全ジョブの履歴
- `karma_logs`: 学習した教訓の蓄積
- `system_settings`: LLM設定、AI名などのシステム設定
- `chat_messages`: Agent Consoleのチャット履歴

### 5.2 DB ファイルの場所
SQLite DB は `workspace/aiome.db` に自動作成されます。

---

## 6. Monitoring (監視)

### 6.1 ログ出力
```bash
RUST_LOG=info cargo run -p api-server
```

---

## 7. Troubleshooting (トラブルシューティング)

| Symptom | Cause | Solution |
|---------|-------|----------|
| `401 Unauthorized` | 認証トークン不一致 | ブラウザをリロードし再認証、`.env` の `API_SERVER_SECRET` を確認 |
| `403 Forbidden` | API キーが無効 | `.env` のキーを確認 |
| Settings画面が開かない | 401エラー | ブラウザタブをリフレッシュして再認証 |
| Ollamaモデル選択不可 | 認証切れ or Ollama未起動 | `ollama serve` を確認、ブラウザリロード |
| 日本語入力でテキストが消えない | IMEバグ (修正済み) | 最新版にアップデート |

---

## 8. Production Deployment Checklist
- [ ] `.env` に `GEMINI_API_KEY` を設定
- [ ] `.env` に `API_SERVER_SECRET` を設定
- [ ] `.env` に `VAULT_SECRET` を設定 (Key Proxy用)
- [ ] `.env` に `FEDERATION_SECRET` を設定 (Samsara Hub用)
- [ ] `SOUL.md` を確認・カスタマイズ
- [ ] Ollama でモデルをダウンロード (`ollama pull qwen3.5:9b`)
- [ ] `cargo run -p api-server` でテスト起動
- [ ] ブラウザで `http://localhost:3015` にアクセスし動作確認

### 9. local Embedding Server (ruri-v3) の起動
1. `tools/ruri-embed-server` に移動。
2. `python3 -m venv venv` で仮想環境作成。
3. `source venv/bin/activate` (Mac/Linux)。
4. `pip install -r requirements.txt`。
5. `python3 server.py` で起動 (デフォルト 8100 ポート)。

---
*Happy coding!*
