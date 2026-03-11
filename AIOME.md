# Aiome Production Template (Golden Rule)

大規模開発に向けた、堅牢でスケーラブルなRustプロジェクト構成。

## 🏗️ アーキテクチャ (Workspace構成)

厳格な依存方向を守ること。

```
apps/api-server  (Main, DIコンテナ)
      ↓
libs/infrastructure (DB, Redis, External API)
      ↓
libs/core           (Domain Logic, Entity, Interface)
      ↓
libs/shared         (Common Utils, Types)
```

- **禁止**: `core` が `infrastructure` に依存すること（依存性逆転の原則）。
- **禁止**: `shared` が他の層に依存すること。

## 🛡️ Iron Principles (鉄の掟)

1.  **Result型強制**: `unwrap()`, `expect()` は `examples/` と `tests/` 以外で禁止。
2.  **型安全性**: 文字列でデータを回さず、NewTypeパターンやEnumを使用する。
3.  **非同期**: `tokio` ランタイムを使用。ブロッキング処理は禁止。
4.  **エラー処理**: `anyhow` (アプリ層) と `thiserror` (ライブラリ層) を使い分ける。

## 🛠️ 利用可能なワークフロー

```bash
/task <件名>    # Issue作成 & 開発開始
/docs-gen <Path> # コードから仕様書生成
/tdd <内容>     # 実装
```

## 📦 ディレクトリ構成

- **apps/**: 実行可能な Rust アプリケーション (Standalone)
  - **api-server**: モノリシックを解消したルーティングサーバー
  - **management-console**: Tauri v2 + React を用いたデスクトップシェル (Dashboard v2)
- **libs/**: 再利用可能なライブラリ群
  - **core**: 純粋なビジネスロジック (no IO ideally)
  - **infrastructure**: I/Oの実装 (SQLite, API Request, ConstitutionalValidator)
  - **shared**: 共通型定義、セキュリティ境界 (Guardrails)

## 🧪 テスト戦略

- **Unit Test**: `core` 内でロジックを徹底的にテスト
- **Integration Test**: `api-server` でエンドポイントをテスト
- **Mocking**: `mockall` を使用して `infrastructure` をモック化
- **TDD Forge (Skill Generation)**: エージェントが新規スキルを作成する際は、必ず本番パブリッシュ前に隔離エンドポイント（`forge_test_run`）でJSON Assertを行うこと。
- **Build Isolation**: エージェントによるローカルコンパイルは必ずOSネイティブサンドボックス（`sandbox-exec` 等）の厳格なプロファイル下で実行すること。
