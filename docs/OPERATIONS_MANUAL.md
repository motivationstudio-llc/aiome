# Aiome Operations Manual — 実用運用ガイド
**Version:** 1.0  
**Last Updated:** 2026-02-22

---

## 1. Prerequisites (前提条件)

### 1.1 Hardware
- **推奨**: Mac mini M4 Pro (24GB RAM) 以上
- **GPU**: Apple Silicon 統合GPU (ComfyUI 用)
- **Storage**: SSD 100GB+ (動画素材蓄積のため)

### 1.2 Software Dependencies

| Software | Version | Purpose | Install |
|----------|---------|---------|---------|
| Rust | 1.75+ | コア開発 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Ollama | Latest | ローカルLLM (脚本生成) | `brew install ollama` |
| ComfyUI | Latest | 画像/動画生成 | [GitHub](https://github.com/comfyanonymous/ComfyUI) |
| FFmpeg | 6.0+ | メディア合成 | `brew install ffmpeg` |
| SQLite | 3.40+ | DB (ビルトイン) | Rust `sqlx` に含まれる |

### 1.3 API Keys (必須)

| Key | 取得先 | 用途 |
|-----|--------|------|
| `GEMINI_API_KEY` | [Google AI Studio](https://aistudio.google.com/apikey) | Oracle (動画評価AI) |
| `BRAVE_API_KEY` | [Brave Search API](https://brave.com/search/api/) | World Context (トレンド検索) |
| `YOUTUBE_API_KEY` | [Google Cloud Console](https://console.cloud.google.com/) | SNS Sentinel (再生数/コメント取得) |

---

## 2. Initial Setup (初期セットアップ)

### 2.1 環境変数の設定

```bash
# プロジェクトルートの .env ファイルを編集
cd /path/to/aiome
cp .env.example .env  # テンプレートがない場合は手動作成

# .env の内容:
GEMINI_API_KEY=あなたのGemini APIキー
BRAVE_API_KEY=あなたのBrave APIキー
YOUTUBE_API_KEY=あなたのYouTube APIキー
COMFYUI_API_URL=ws://127.0.0.1:8188/ws
COMFYUI_BASE_DIR=/path/to/ComfyUI
EXPORT_DIR=/path/to/exports
WORKSPACE_DIR=./workspace
# --- Karma Federation (任意, マルチノード展開時に設定) ---
FEDERATION_SECRET=認証用のお好みの秘密キー
FEDERATION_PEERS=https://peer1.example.com,https://peer2.example.com
```

### 2.2 ビルドと初期検証

```bash
# 1. ビルド
cargo build --release -p shorts-factory

# 2. テスト実行 (全50テストが通過することを確認)
cargo test --workspace

# 3. 進化シミュレーションの実行 (API接続テスト)
cargo run -p shorts-factory -- simulate-evolution
```

成功すると以下のような出力が表示されます:
```
🏁 --- [Evolution Simulation] --- 🏁
🔮 Oracle is evaluating...
⚖️  Verdict:
   - Topic Score:  0.85
   - Visual Score: 0.00
   - Soul Score:   0.80
🧬 Simulated Karma Weight: 67 / 100
🏁 --- [Simulation Complete] --- 🏁
```

### 2.3 ワークスペースの準備

```bash
# ワークフロー定義ディレクトリ
mkdir -p workspace/config
mkdir -p workspace/workflows

# skills.md の作成 (AI が参照する能力定義)
cat > workspace/config/skills.md << 'EOF'
# Available Skills
- tech_news_v1: テクノロジーニュース解説
- cyberpunk_v1: サイバーパンク映像
# 必要に応じてスタイルを追加
EOF
```

---

## 3. Commands (コマンド一覧)

### 3.1 動画生成 (手動)

```bash
cargo run -p shorts-factory -- generate --category tech
```

### 3.2 自律モード起動 (Cron Scheduler)

```bash
cargo run -p shorts-factory -- serve
```

これにより以下の8つの自動ジョブが起動します:

| Job | Schedule | Function |
|-----|----------|----------|
| **Samsara** | Daily 07:00/19:00 | RAG駆動のジョブ自動生成 |
| **Zombie Hunter** | Every 15min | ハングしたジョブの回収 |
| **Tech Distiller** | Every 30min | 実行ログからの教訓抽出 |
| **Creative Distiller** | Every 30min | 人間フィードバックの反映 |
| **File Scavenger** | Daily 03:00 | 古い一時ファイルの清掃 |
| **DB Scavenger** | Daily 03:30 | 古いDBレコードの清掃 |
| **Sentinel** | Every 4h | SNSメトリクス収集 |
| **Oracle** | Every 1h | AI評価 (最終審判) |
| **Karma Distiller** | Daily 04:00 | 記憶の圧縮 (Day-2防壁) |
| **Adaptive Immune System** | Every 1h | 脆威ログ分析と新規防衛ルールの自動生成 |
| **Karma Federation Sync** | Every 5min (+Jitter) | ピアノードへのKarma同期（`FEDERATION_PEERS`設定時） |

### 3.3 SNS リンク (手動)

動画を YouTube にアップロード後:
```bash
cargo run -p shorts-factory -- link-sns --job-id <JOB_UUID> --platform youtube --video-id <YOUTUBE_VIDEO_ID>
```

### 3.4 進化シミュレーション

```bash
cargo run -p shorts-factory -- simulate-evolution
```

---

## 4. Configuration (設定)

### 4.1 `config.toml` (プロジェクトルート)

```toml
ollama_url = "http://localhost:11434/v1"
comfyui_api_url = "ws://127.0.0.1:8188/ws"
model_name = "qwen2.5-coder:32b"
batch_size = 10
comfyui_timeout_secs = 180
clean_after_hours = 24
```

### 4.2 `SOUL.md` (AIの人格定義)

プロジェクトルートの `SOUL.md` を編集すると、Oracle の評価基準と Samsara の生成方針が変化します。  
**⚠️ 変更する場合はバックアップを取ってから行ってください。**

### 4.3 `styles.toml` (演出スタイル定義)

動画の演出パラメータ (カメラワーク、BGM音量、ダッキング等) を定義します。

---

## 5. Database (データベース)

### 5.1 スキーマ概要

```
jobs               ← 全ジョブの履歴 (不滅のスキーマ)
karma_logs         ← 学習した教訓の蓄積 (進化の記憶)
sns_metrics_history ← SNS評価の時系列データ (評価台帳)
```

### 5.2 DB ファイルの場所

SQLite DB は `workspace/aiome.db` に自動作成されます。  
WAL モードで動作し、読み書きの並行処理に対応しています。

### 5.3 バックアップ

```bash
# 日次バックアップ (WAL モードのため、-wal / -shm ファイルも含めること)
cp workspace/aiome.db workspace/aiome.db-wal workspace/aiome.db-shm /path/to/backup/
```

---

## 6. Monitoring (監視)

### 6.1 ログ出力

```bash
# 詳細ログで起動
RUST_LOG=debug cargo run -p shorts-factory -- serve

# 通常運用 (INFO レベル)
RUST_LOG=info cargo run -p shorts-factory -- serve
```

### 6.2 Watchtower (Discord 通知)

`apps/watchtower` を起動すると、ジョブ完了/失敗を Discord に自動通知します。

### 6.3 Command Center (WebUI)

```bash
cd apps/command-center
npm run dev  # Tauri GUI の開発起動
```

---

## 7. Troubleshooting (トラブルシューティング)

| Symptom | Cause | Solution |
|---------|-------|----------|
| `403 Forbidden` (Oracle) | Gemini API キーが無効 | `.env` の `GEMINI_API_KEY` を確認 |
| `NOT NULL constraint failed: jobs.karma_directives` | 古い DB スキーマ | DB を削除して再起動 (マイグレーション自動実行) |
| `Poison Pill Activated` ログ | 3回連続API失敗 | API キー/クォータを確認。該当ジョブは自動停止済 |
| Oracle が無応答 | トークン量オーバー | Karma Distiller が自動圧縮を行う (毎日04:00)。手動実行不要 |
| ComfyUI 接続エラー | ComfyUI が起動していない | `python main.py` で ComfyUI を先に起動 |
| ジョブが `Processing` のまま | ゾンビ化 | Zombie Hunter が15分ごとに自動回収 |

---

## 8. Production Deployment Checklist

- [ ] `.env` に全API キーを設定
- [ ] `SOUL.md` を確認・カスタマイズ
- [ ] `workspace/config/skills.md` にスタイルを定義
- [ ] `workspace/workflows/` にComfyUIワークフローJSONを配置
- [ ] Ollama でモデルをダウンロード (`ollama pull qwen2.5-coder:32b`)
- [ ] ComfyUI を起動
- [ ] `cargo run -p shorts-factory -- simulate-evolution` で接続テスト
- [ ] `cargo run -p shorts-factory -- serve` で自律モード開始
- [ ] (Optional) `apps/watchtower` で Discord 監視を有効化
- [ ] (Optional) `.env` に `FEDERATION_SECRET` と `FEDERATION_PEERS` を設定してKarmaフェデレーションを有効化

---

## 9. Version Upgrade (バージョンアップ手順)

### DBマイグレーションの安全性
データベーススキーマの変更（テーブル追加等）はすべて `init_db()` 内の `CREATE TABLE IF NOT EXISTS` 構文で管理されています。起動時に自動反映されるため、マイグレーションツールは不要です。

```bash
# バージョンアップ手順
# 1. 安全のため、先にDBをバックアップ
cp workspace/aiome.db workspace/aiome.db.bak

# 2. 最新のバイナリをビルド
cargo build --release -p shorts-factory

# 3. サービスを再起動 (スキーマは init_db() で自動適用)
cargo run -p shorts-factory -- serve
```
