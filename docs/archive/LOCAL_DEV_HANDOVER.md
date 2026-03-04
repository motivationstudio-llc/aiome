# 🤖 Qwen 3 Coder ローカル開発用指示書

このドキュメントは、ローカル LLM (Qwen 3 Coder 等) を使って ShortsFactory の機能を実装するためのプロンプト集です。

## � 憲法（必ず最初に読み込ませる）

全てのモジュール開発に先立ち、Qwen に以下の「鉄の掟」を厳守させてください：

```markdown
# Rust Module Development Guidelines (Strict Mode)

あなたは堅牢なRustモジュールを開発するエンジニアです。以下のルールを絶対に守ってください。

1. **エラー処理**: `unwrap()`, `expect()` は禁止。`factory_core::error::FactoryError` を使用する。
2. **Schema Safety**: JSONパース失敗時は `shared::output_validator` を使い、LLMに修正を依頼する再試行ロジックを含めること。
3. **File System Sandbox**: 全てのファイル操作は `shared::sandbox::PathSandbox` で検証すること。`canonicalize()` + プレフィックスチェックを必ず通す。
4. **Zombie Killer**: 外部プロセス（ComfyUI, FFmpeg）呼び出しは必ず `shared::zombie_killer::run_with_timeout` を使用（デフォルト300秒）。タイムアウト時はプロセスを `kill` すること。
5. **Cold Start Awareness**: 初回のモデルロード時はタイムアウトを延長（例: 600秒）するか、`/system_stats` で準備完了を確認すること。
6. **Maintenance**: 1サイクルごとに `shared::cleaner::StorageCleaner` でゴミ掃除を行い、ディスク残量を確認すること。
7. **No Sleep**: 起動時に `shared::os_utils::prevent_app_nap()` を呼び出し、Mac が眠らないようにすること。
8. **Testing First**: コードを書く前に、正常系と異常系のテストケースを作成すること。`cargo test` が通らないコードは提案しないこと。
9. **非同期**: `tokio` と `async_trait` を使用。ブロッキング処理は禁止。
10. **命名**: `factory_core`（`factory-core` の Cargo alias）を使用すること。
```


---

## 📄 タスク別の指示内容

### 1. `config.toml` の読み込み実装
**対象ファイル:** `libs/shared/src/config.rs`

**指示プロンプト:**
> `libs/shared/src/config.rs` を編集して、カレントディレクトリの `config.toml` から設定を読み込む機能を追加してください。`serde` と `toml` クレート（なければ追加）を使用してください。
> - `FactoryConfig::load()` メソッドを実装。
> - ファイルがない場合はデフォルト値を返す。

### 2. ComfyBridge (HTTP API) の実装
**対象ファイル:** `libs/infrastructure/src/comfy_bridge.rs`

**指示プロンプト:**
> `libs/infrastructure/src/comfy_bridge.rs` を完成させてください。
> - `factory_core::traits::VideoGenerator` を実装。
> - `shared::zombie_killer::http_client_with_timeout` でタイムアウト付き HTTP クライアントを使うこと。
> - `POST /prompt` でワークフローを実行し、返ってきた `prompt_id` でポーリング待機。
> - ポーリングにもタイムアウト（デフォルト300秒）を設定し、超過時は `FactoryError::ComfyTimeout` を返す。
> - 実装前に `libs/core/src/traits.rs` と `libs/core/src/error.rs` を参照すること。

### 3. MediaForge (FFmpeg) の実装
**対象ファイル:** [NEW] `libs/infrastructure/src/media_forge.rs`

**指示プロンプト:**
> `libs/infrastructure/src/media_forge.rs` を新規作成してください。
> - `factory_core::traits::MediaEditor` を実装。
> - FFmpeg の実行には必ず `shared::zombie_killer::run_with_timeout` を使用すること（デフォルト300秒）。
> - 出力ファイルのパスは `shared::sandbox::PathSandbox` で検証すること。
> - 動画、音声、字幕 (ASS/SRT) の合成、および 9:16 (1080x1920) へのリサイズ。
> - エラー時は `FactoryError::FfmpegFailed` を返すこと。

### 4. FactoryLog (SQLite) の実装
**対象ファイル:** [NEW] `libs/infrastructure/src/factory_log.rs`

**指示プロンプト:**
> `libs/infrastructure/src/factory_log.rs` を新規作成してください。
> - `factory_core::traits::FactoryLogger` を実装。
> - `sqlx` を使って SQLite に履歴を保存。
> - テーブル名: `production_logs` (id, video_id, path, created_at)。
> - データベースファイル名: `factory.db`。

---

## 🚫 注意：Antigravity（私）に渡すべきタスク

以下の作業は Qwen には難易度が高すぎるため、私が担当します：

1. **`rig-core` の `Tool` トレイト実装**: マクロやジェネリクスが複雑なため。
2. **`shorts-factory/src/main.rs` での統合**: エージェントの思考ロジックとツールの接続。
3. **ワークスペース全体の修正**: ビルドエラーが多発した場合の解決。

---

## 💡 Qwen で開発するときのヒント
- **エラーが出たら**: エラーメッセージをそのまま Qwen に貼り付け、「Iron Principles に従って修正して」と言ってください。
- **型がわからない時**: `factory-core` の該当ファイル（`traits.rs` や `error.rs`）の中身をプロンプトに含めてあげてください。
