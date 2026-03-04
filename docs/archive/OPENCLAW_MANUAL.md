# 🦞 Aiome 活用マニュアル: ローカルAIエージェントの運用

本ドキュメントでは、本プロジェクトにおける Aiome の活用方法について解説します。

## 1. 基本コマンド

Aiomeは CLI (`npx openclaw@latest`) を通じて操作します。

### エージェントとの対話
```bash
# 基本的な質問
npx openclaw@latest agent --message "プロジェクトの構造を説明して"

# モデルを指定して実行 (高速な応答が必要な場合)
npx openclaw@latest agent --message "Hello" --model ollama/llama3.1:8b
```

### システムの状態確認
```bash
# デバイスとプロバイダーのステータスを確認
npx openclaw@latest doctor

# 使用可能なモデルの一覧を表示
npx openclaw@latest models list
```

## 2. Ollamaの管理

Aiomeはローカルの Ollama に依存しています。

```bash
# 現在動作中のモデルを確認
ollama ps

# 新しいモデルを追加
ollama pull qwen2.5-coder:32b
```

## 3. スキル（Skills）の活用

`.agent/skills` ディレクトリにあるフォルダは、エージェントが実行可能な「スキル」として認識されます。

### 既存のスキル
- **backend-patterns**: Rustエンタープライズパターンの適用。
- **frontend-patterns**: 管理画面などのUI開発標準。
- **coding-standards**: コード品質の維持。

### 新しいスキルの作成
1. `.agent/skills/new-skill/SKILL.md` を作成。
2. そのスキルで何ができるかを記述。

## 4. ワークフロー（Workflows）の実行

`.agent/workflows` に定義されたスラッシュコマンドを使用できます。

- `/plan`: 実装前に計画を作成。
- `/docs-gen`: 特定のモジュールからドキュメントを生成。
- `/tdd`: テスト駆動開発の実行。

## 5. 管理画面との連携

`api-server` を起動すると、ブラウザからナレッジベースを確認できます。

```bash
cd apps/api-server
cargo run
```
👉 `http://localhost:3015`

## 6. 注意事項
- **リソース消費**: `llama4:latest` などの巨大なモデルを動かす際は、メモリ消費に注意してください。反応がない場合は `ollama ps` で確認してください。
- **APIキー**: Web検索（Brave Search）などを使用する場合は、別途 `openclaw configure --section web` でキーを設定する必要があります。
