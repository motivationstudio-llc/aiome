---
description: GitHub Issueに基づいたタスク実行。Issue確認 → 計画 → 実装 → PR作成 → Issue更新までの一連のフロー。
---

# /task - GitHub駆動開発ワークフロー

GitHub Issueを起点として開発を進めるワークフローです。
コンテキストの喪失を防ぎ、GitHubを「プロジェクトの真実」として常に最新に保ちます。

## コマンド使用法

```bash
# 特定のIssueに着手
/task #123

# 新しいタスクを開始（Issue作成から）
/task 新しい機能を追加したい
```

## 実行フロー

### 1. タスク確認・準備 (GitHub MCP)
- **指定がIssue番号の場合**:
  - Issueの内容、コメント、現在のステータスを取得します。
  - 既にPRがあるか、作業中のブランチがあるかを確認します。
- **指定が新規内容の場合**:
  - 新規Issueを作成し、その番号を取得します。

### 2. コンテキスト同期
- 最近の変更や関連するIssueを検索し、重複実装を防ぎます。
- `AIOME.md` や `rules.md` のルールを確認します。

### 3. 実装計画 (GitHub MCP)
- 複雑なタスクの場合、実装計画を立案します。
- **アクション**: 計画をIssueのコメントとして投稿します（記憶の外部化）。
  > 📝「以下の計画で実装を進めます...」

### 4. 実装 & テスト (Local)
- トピックブランチを作成します (`feat/issue-123-...`)。
- `/tdd` ワークフローに従って実装とテストを行います。
- `git commit` でコミットします（メッセージに `Close #123` 等を含めない）。

### 5. レビュー & PR作成 (GitHub MCP)
- lintやテストが通ることを確認します。
- **アクション**: PRを作成し、Issueに関連付けます。
- PRの説明には、変更内容の要約とIssueへのリンクを含めます。

### 6. 完了報告 (GitHub MCP)
- **アクション**: Issueに作業完了のコメントを投稿します。
- 必要に応じてIssueのステータスを更新します（例: In Progress → Review）。

## GitHub MCP 活用ポイント

以下のアクションは **即座にGitHub上に反映** されます：

- `create_issue`: Issue作成
- `get_issue`: Issue内容取得
- `add_issue_comment`: コメント投稿（進捗/計画メモ）
- `create_pull_request`: PR作成
- `search_issues`: 関連タスク検索

## コミットメッセージ規約

Issue番号を紐付けるため、以下の形式を推奨します：

```
feat: ユーザー認証機能を追加 (#123)
fix: ログイン時のクラッシュを修正 (#124)
```

## 事前準備

`mcp_config.json` にGitHub設定が必要です：

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "YOUR_TOKEN"
      }
    }
  }
}
```
