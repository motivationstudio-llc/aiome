# Aiome Operations Manual — 実用運用ガイド
**Version:** 1.0  
**Last Updated:** 2026-03-05

---

## 1. Prerequisites (前提条件)

### 1.1 Hardware
- **推奨**: Mac mini M4 Pro (24GB RAM) 以上
- **Storage**: SSD 10GB+ (データ蓄積のため)

### 1.2 Software Dependencies

| Software | Version | Purpose | Install |
|----------|---------|---------|---------|
| Rust | 1.75+ | コア開発 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Ollama | Latest | ローカルLLM | `brew install ollama` |
| SQLite | 3.40+ | DB (ビルトイン) | Rust `sqlx` に含まれる |

### 1.3 API Keys (オプション)

| Key | 取得先 | 用途 |
|-----|--------|------|
| `ORACLE_API_KEY` | [API Provider] | 高度なプランニング / 評価用 |

---

## 2. Initial Setup (初期セットアップ)

### 2.1 環境変数の設定

```bash
# プロジェクトルートの .env ファイルを編集
cp .env.example .env

# .env の内容:
ORACLE_API_KEY=あなたのAPIキー
EXTERNAL_SERVICE_URL=ws://127.0.0.1:8188/ws
WORKSPACE_DIR=./workspace
```

### 2.2 ビルドと初期検証

```bash
# 1. ビルド
cargo build --release -p aiome-daemon

# 2. テスト実行
cargo test --workspace
```

---

## 3. Commands (コマンド一覧)

### 3.1 デーモン起動

```bash
cargo run -p aiome-daemon
```

---

## 4. Configuration (設定)

### 4.1 `styles.toml` (演出スタイル定義)
必要に応じて生成スキルのパラメータを定義します。

---

## 5. Database (データベース)

### 5.1 スキーマ概要
- `jobs`: 全ジョブの履歴
- `karma_logs`: 学習した教訓の蓄積
- `sns_metrics_history`: 外部評価の時系列データ

### 5.2 DB ファイルの場所
SQLite DB は `workspace/aiome.db` に自動作成されます。

---

## 6. Monitoring (監視)

### 6.1 ログ出力
```bash
RUST_LOG=info cargo run -p aiome-daemon
```

---

## 7. Troubleshooting (トラブルシューティング)

| Symptom | Cause | Solution |
|---------|-------|----------|
| `403 Forbidden` | API キーが無効 | `.env` のキーを確認 |
| Connection Error | 外部エンジンが未起動 | 対象エンジンを起動 |
| ジョブが `Pending` のまま | キューの詰まり | デーモンを再起動 |

---

## 8. Production Deployment Checklist
- [ ] `.env` に API キーを設定
- [ ] `SOUL.md` を確認・カスタマイズ
- [ ] Ollama でモデルをダウンロード
- [ ] `cargo run -p aiome-daemon` でテスト起動

---
*Happy coding!*
