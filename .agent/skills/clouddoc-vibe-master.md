---
name: clouddoc-vibe-master
description: Documentationを活用して、大規模ボイラープレート環境での「バイブコーディング」を爆速化・堅牢化するスキル。
---

# 🌌 Documentation Vibe Master

Aiome（大規模ボイラープレート）環境において、ノリと直感（Vibe）で開発しながらも、構造を見失わず、かつ高品質なドキュメントを自動生成し続けるためのスキルです。

## 🎯 目的
- **思考の同期**: 複雑なAiomeの構造をAIに完全に把握させ、開発者の脳をハックする。
- **デバッグの高速化**: 変更が他にどこに影響するかをDocumentationに一瞬で分析させる。
- **ドキュメントの「ゼロ・コスト」化**: 開発者がコードを書くだけで、最高品質のWikiが自動生成される状態を維持する。

## 🛠️ 基本メソッド

### 1. 実装前の「構造プレビュー」
新しい機能を実装する前に、必ずDocumentationに構造を問いかけ、実装の「ノリ」を決定します。
- **アクション**: `grep` や `CLOUD_DOCUMENTATION.md` を参照し、類似の既存パターンを特定。
- **指示例**: 「新しいStripe課金プランを追加したい。既存のSubscriptionモデルとの関連図をMermaidで出して。」

### 2. コミット前の「ドキュメント・セルフ更新」
プッシュする前に、ローカルでWikiを更新し、自分の変更が正しく図解されるか確認します。
- **コマンド**: `python3 scripts/generate_docs.py`
- **チェック**: 管理画面 (`localhost:3000`) で新しいモジュールが反映されているか、依存関係が設計通りかを目視。

### 3. Vibeガード（デストラクティブ変更の防止）
大規模リファクタリング時、Documentationのコンテキストを利用して「壊してはいけない場所」を特定します。
- **アクション**: `libs/core` の変更が `libs/infrastructure` に波及していないか、Documentationの依存関係図でチェック。

## 🔄 開発プロセスへの組み込み

1.  **[設計]**: Documentationに「この機能を作るならどのファイルを触るべき？」と聞き、バイブスを合わせる。
2.  **[実装]**: TDDやVibe Codingで爆速実装。
3.  **[Sync]**: `scripts/generate_docs.py` を実行し、ドキュメントの True 状態を確認。
4.  **[Push]**: GitHub Actions経由でDocumentation Cloudを更新。

## 📝 鉄の掟 (Required Reading)
- `docs/DOCUMENTATION_USAGE_GUIDE.md` を常にプロジェクトの「真実」として扱う。
- Doc Comments (`///` or `/** */`) を書くことは、将来の自分への最高の投資である。

---
> 開発者はコードに集中し、構造の記憶と説明は Documentation Vibe Master が引き受ける。
