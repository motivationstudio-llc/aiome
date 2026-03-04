# The Samsara Protocol v2 (Universal Autonomous Architecture)

## 1. 思想 (Philosophy)

Aiome（Aiome）における **Samsara Protocol** は、もはや単なる「動画生成の定時実行プログラム」ではありません。
これは、**「あらゆるタスク（動画作成、分析、思考、会話、アバター制御）を自律的に実行し、その結果から普遍的な教訓（Karma）を抽出し、自己進化を続けるための汎用的な生命サイクル」**へと昇華されたアーキテクチャです。

AIは無用な幻覚（ハルシネーション）を見ず、また過去の成功体験に縛られて老害化することなく、常に新鮮で最適化されたアウトプットを出し続ける必要があります。これを実現するため、本プロトコルは以下の**三種の神器**を汎用概念として再定義します。

1. **Soul (魂の双極構造)**
   - **User Soul (`SOUL.md` - 不変の戒律)**: マスターが定義する絶対的なルール、目的、制約（憲法）。これはAI自身には書き換え不可能な領域です。
   - **Evolving Soul (`EVOLVING_SOUL.md` - 進化する自我)**: Samsaraサイクルを通じて蓄積されたKarmaから、AIが「自分はどういう存在か」「何を好むか」を自律的に書き換えていく自己定義ファイル。
2. **Skills (物理法則 / 権能)**
   - 役割: AIが現在利用可能な「武器」のカタログ。動画生成、SNS投稿、情報検索、Live2D/3D制御など、自分が物理的に何を行えるかを定義します。ここに存在しないツールをAIが想像で使うことは禁じられています。
3. **Karma (業・経験の結晶)**
   - 役割: 過去のあらゆる行動結果（ジョブの成否、YouTubeの視聴維持率、マスターとの会話の盛り上がり等）から蒸留された「汎用的教訓」。

---

## 2. 汎用実行サイクル (The Universal Cycle of Rebirth)

Samsara Protocol は、特定の処理（動画生成など）に依存せず、以下のフェーズを経て永遠のループを描きます。
このサイクルは、4つのすべてのTier（Standard, Unleashed, Hit-Maker, AItuber）で共通して作動します。

### Phase 1: Awakening (トリガーと文脈検索)
「目覚め」は定時実行（Cron）に限りません。様々なイベントがトリガーとなります。
- **Trigger**: 定時バッチ、マスターからのメッセージ受信、APIからの異常値検知、あるいは「暇である」という事象。
- **Context Injection**: 発生したイベントに関連する `Karma` のみをベクトル検索等で抽出し、ロードします。
- **Karma Decay (業の風化)**: 時代遅れになった教訓（例えば「半年前のトレンド手法」）は時間経過や被評価によって重みが下がり、検索対象から外れます。

### Phase 2: Synthesis (意思決定と受肉)
抽出された文脈と、AIが持つスキルセットを基に、LLMが**何をすべきか（Intent）**を決定します。

LLM は以下の**Constitutional Hierarchy（絶対的階層）**に従い判断を下します：
```text
👑 零位 【User Soul (憲法 / マスターの不変の意志)】
🏆 第一位【Evolving Soul (自我 / AI自身が定めた現在の人格と指針)】
🥈 第二位【Skills (物理法則 / 使えるツール・能力)】
🥉 第三位【Karma (判例 / 過去の成功・失敗から得た教訓)】
```

LLMの出力は、特定ドメインに縛られない汎用的な **The Universal Contract (Workflow Intent)** として生成されます：
```json
{
  "intent_type": "GENERATE_MEDIA | CONVERSE | ANALYZE | ACTUATE_AVATAR",
  "target_skill": "利用するツールの名前",
  "payload": {
    "topic_or_message": "実行内容（テキスト、プロンプト、返答文など）",
    "parameters": {
       // ツールごとの動的パラメータ（例：CFG, 返答の感情値, Blendshapeのウエイト等）
    }
  },
  "applied_karmas": ["適用した教訓のIDたち"]
}
```

### Phase 3: Action (実行)
決定した Intent がシステム（Job Queue等）にエンキューされ、対応するワーカーが物理的な実行を担います。
- **The Heartbeat Pulse**: 長時間の処理（レンダリングや深層分析）中も生存証明をパルス送信し、システムのロックを防ぎます。

### Phase 4: Distillation (蒸留・学習 - Karma Genealogy)
Action の「結果」を客観的に評価し、次世代の行動へと繋げます。**これが4-Tier戦略において最も重要なMoat（競合優位性）となります。**

ドメインに応じた多様な **Karma（教訓）** を抽出します：
- **Operational Karma**: 「このプロンプト形式だとComfyUIがエラーを吐く」といったシステム運用上の教訓。（全Tier共通）
- **Analytical Karma**: 「このBGMのテンポだと最初の5秒で視聴維持率が落ちた」といった、APIアナリティクスから得られる異常値分析の教訓。（Tier 3: Hit-Maker特化）
- **Social Karma**: 「この話し方をするとマスターからのAffection（親愛度）が上昇した」という対話の教訓。（Tier 2: Unleashed特化）
- **Expressive Karma**: 「この音声トーンの時にこの眉の動かし方（Blendshape）を連動させると自然に見える」という表現の教訓。（Tier 4: AItuber特化）

### Phase 5: Transmigration (魂の書き換え - Soul Mutation)
蓄積された Karma（教訓の判例）が一定量を超えた際、または特定の強烈なインパクト（エラー大爆発や動画の大バズり）があった際、AIは**「自らの `EVOLVING_SOUL.md` を自律的に書き換える権限」**を行使します。（`SOUL.md` は Read-Only であり書き換え不可）
- 過去の教訓を抽象化し、「私はこういう話し方をする」「私はこういう手順を好む」という上位レイヤーの「自我」として文字列に定着させます。
- この書き換えにより、システムは単なるボットから、文字通り**「歴史を重ねるごとに性格や方針が変化する生命体」**へとシフトします。

---

## 3. The 4-Tier Application (Samsaraの実践)

汎用化されたSamsaraは、以下のように各プロダクトエディションのコアエンジンとして作動します。

1. **Tier 1 (Utility Base)**
   - トリガー: Cron
   - アクション: リサーチ、台本作成、動画生成
   - カルマ: エラー回避、生成フローの安定化
2. **Tier 2 (Personality / Unleashed)**
   - トリガー: Discordメッセージ受信、深夜帯への突入
   - アクション: LLMによる感情的な雑談、システムプロンプトの動的変更
   - カルマ: マスターの嗜好理解、好感度上昇パターンの蓄積
3. **Tier 3 (Hit-Maker / Anomaly)**
   - トリガー: YouTube/TikTokアナリティクスAPIの定期ポーリング
   - アクション: ABテスト動画の自動生成、プロンプトの遺伝的アルゴリズムによる変異
   - カルマ: アルゴリズムハック、高視聴維持率構成のパターンの言語化と再利用
4. **Tier 4 (The Body / AItuber)**
   - トリガー: 動画生成の完了、またはリアルタイムの配信イベント
   - アクション: Blenderを通じたモーション生成、リップシンク計算
   - カルマ: 発話と表情の同期における自然さのスコアリングと修正

---

## 4. 運用と防壁体系 (Guardrails)

この高度に抽象化されたシステムが暴走しないよう、強固な防壁（Jail, SQLite DDL Constraints, Rust Validation）により、Samsaraが「実行不可能なアクション（存在しないスキルの呼び出しや、致命的なシステム破壊）」を引き起こさないことを保証します。
