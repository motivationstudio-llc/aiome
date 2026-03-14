# コントリビューションガイド (CONTRIBUTING)

## 1. コミット規約
[Conventional Commits](https://www.conventionalcommits.org/) を採用しています。
- `feat`: 新機能
- `fix`: バグ修正
- `docs`: ドキュメントのみの変更
- `refactor`: コード変更（機能・修正なし）
- `test`: テスト追加・変更

## 2. 開発フロー
1. `main` ブランチから機能ブランチ（`feat/some-feature`）を作成。
2. コードを書き、テストを通す（`cargo test`）。
3. ドキュメントが必要な場合は `docs/` を更新。
4. PR を作成し、CI がパスすることを確認。
5. 創業者によるコードレビューを受け、Approve を得る。

## 3. レビュー基準
- `unwrap()` / `expect()` をパブリックAPIで使用していないか。
- `missing_docs` 警告が出ていないか。
- ファイルヘッダーにライセンス（BSL 1.1）が記載されているか。
- `build.rs` を新規追加・変更していないか（セキュリティ上の重要項目）。

## 4. セキュリティ
脆弱性を発見した場合は、issue ではなく、[CTOへの連絡手段] へ直接報告してください。
