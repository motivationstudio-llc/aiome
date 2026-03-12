# AI Agent Evolution Guide

Aiomeの自律的な管理機能と、個々のノードが独自に獲得する「人格（SOUL）」の進化プロセスに関するドキュメントです。

---

## 🏗️ システムの基盤 (Aiome Foundation)

Aiome は、完全なオープンソースとして提供されるエージェント OS です。エンタープライズ級のプロキシセキュリティ（Abyss Vault）や自己防衛、自己進化（Karma）の仕組みが標準ですべて統合されています。
特定の高度なドメインスキル（WASMスキル）は、拡張モジュールとして柔軟に追加可能です。

## 🧘 論理と人格の分離 (Dual-Layer Architecture)

システムの安定性と柔軟性を両立するため、論理層と人格層を分離しています。

1.  **Command Layer (Gemini Cloud / Front-end)**:
    - **役割**: ユーザー応答、Agent Consoleでの対話、高度な論理推論。
    - **特徴**: Gemini 2.5 Flash を使用し、高速かつ低コストなレスポンスを提供。
2.  **Personality Layer (Ollama / Background)**:
    - **役割**: 自律タスク（Soul Mutator、脅威分析、免疫システム）。
    - **特徴**: ローカルの Ollama (qwen3.5:9b) を使用し、無料でバックグラウンド実行。

---

## ⚙️ 進化パラメーター (Evolution Stats)

SQLiteの `agent_stats` テーブルで管理される指標です。

- 💖 **Resonance (共鳴度)**: ユーザーとの良好な対話で上昇。
- ⚙️ **Tech Level (技術力)**: ジョブの成功や複雑な問題の解決で上昇。システムの自律的な提案頻度や精度に影響します。

---

## 🌐 集合知ネットワーク (Karma Federation)

単一ノードでの学習（Karma）や防衛ルール（Immune Rules）は、環境変数 `FEDERATION_PEERS` と `FEDERATION_SECRET` を設定することで、複数のノード間でリアルタイムに同期されます。

- **The Auth Wall**: 厳格な認証により、悪意ある外部からの学習データの「毒入れ」を防止します。
- **群体としての進化**: 1つのノードが未知のエラーに遭遇して生成した「免疫ルール」や、タスクごとの「技術的教訓」は瞬時にクラスタ全体に伝播し、全体の防衛力と創造性を引き上げます。

---

最終更新: 2026-03-13
Aiome Development Team
