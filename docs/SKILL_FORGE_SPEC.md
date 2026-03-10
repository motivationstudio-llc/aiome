# Skill Forge (自律技能鍛造) 仕様書 (v1.0)

## 概要

`Skill Forge` は、Watchtower が自らの能力が不足していると判断した際に、リアルタイムで Rust コードを書き下ろし、WASM (WebAssembly) プラグインとしてコンパイル・抽出し、即座に自身の「機能（Skill）」として統合するための自己進化システムである。

---

## 🏗️ システムアーキテクチャ

### 1. 自己進化サイクル (Auto-Evolution Loop)

1.  **意図解析 (Parse)**: `CommandCenter` (Oracle LLM) がユーザーの要求を解析し、既存スキルで対応不能かつ「構築可能」な場合に `forge_skill` 判定を下す。
2.  **設計と実装 (Forge)**: `SkillForge` が Oracle LLM に対して、特定の PDK 制約と「鉄の掟」を遵守した Rust コードの生成を依頼する。
3.  **隔離ビルド (Compile)**: 生成されたコードは `/tmp` 下の UUID フォルダで、最小限の依存関係を持つ `skill_generator` テンプレートを用いて `wasm32-wasip1` ターゲットでビルドされる。
4.  **検証とデプロイ (Load)**: ビルドが成功すると、WASM ファイルは `workspace/skills/` に配置され、`WasmSkillManager` がこれをホットロードする。
5.  **実行 (Execute)**: ロードされたスキルは WASI サンドボックス内で、秒単位のタイムアウトとメモリ制限、ネットワークホワイトリスト管理の下で実行される。

### 2. 構成コンポーネント

| コンポーネント | 役割 |
| :--- | :--- |
| **`WasmSkillManager`** | WASM プラグインの実行寿命、サンドボックス（WASI）、リソース制限を管理。 |
| **`SkillForge`** | LLM による Rust コード生成と `cargo` を用いたコンパイルプロセスを制御。 |
| **`McpProcessManager`** | 外部のプログラミング言語（Node.js, Python 等）で書かれた MCP サーバープロセスのライフサイクル（PGID ゾンビキル）と標準入出力を管理。 |
| **`DockerDelegator`** | WASM の制限を超える重い依存関係や信頼できない複雑なタスクを、使い捨ての Docker Agent コンテナ（Shadow Worker）へ安全に委譲（Delegation）する。 |
| **`SKILL_FORGE_PROMPT.md`** | 高品質で安全なプラグインを生成するための「鉄の掟」を定めたシステム構成済プロンプト。 |

---

## 🛡️ ゼロトラスト 7 層防衛 (Defense in Depth)

1.  **ランタイム分離**: Extism (Wasmtime) によるメモリ・CPU レベルの物理隔離。
2.  **ビルド制限**: スキル生成時に `build.rs` を強制排除し、ホスト OS での任意コード実行を防止。
3.  **ファイルシステム隔離**: WASI `preopen` により、スキルは `/mnt` (Jail root) 以外へのアクセスが不可能。
4.  **ネットワーク制御**: `AllowedHosts` 設定により、意図したドメイン以外への通信を遮断。
5.  **リソース上限**: メモリ使用量上限 (100MB) と実行時間上限 (10秒) を適用。
6.  **機密情報隠蔽**: プラグインはホストの環境変数に直接触れることはできず、許可されたキーのみが WASM メモリへ注入される。
7.  **出力バリデーション**: スキルの生出力は Oracle LLM (Synthesis層) が必ず再解析し、不適切な情報をユーザーに返さない。

---

## 🧪 鍛造されたスキルの例

- **`crypto_price_fetcher`** (WASM): 外部 API から仮想通貨のリアルタイム価格を取得する（軽量・高速）。
- **`calculator_pro`** (WASM): 複雑な金融計算や数式処理を行う。
- **`docker_agent_worker`** (Delegation): WASM では実行できないブラウザ自動化（Playwright 等）や機械学習モデルの推論など、ホスト環境を汚染するタスクを Docker Agent 経由で実行。

---

## 🛠️ トラブルシューティングと自己修復

- **Compilation Error**: ビルド失敗時、`SkillForge` は最大 3 回まで、エラー内容を LLM にフィードバックしてコードの自動修正（Self-Healing）を試みる。
- **Execution Fail**: 実行エラー（タイムアウト等）が発生した場合、Watchtower はユーザーに対し、現在機能が不安定であることを丁寧に報告し、次の鍛造サイクルでの修正を期す。

---

更新日: 2026-03-03
管理者: Aiome / Watchtower Evolution Unit
