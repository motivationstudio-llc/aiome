# Lex AI Constitution (AI都市建築基準法)

本ドキュメントは、Modular OpenClaw プロジェクトにおいて AI エージェント（アクター）を「自律的な部品」として安全かつ堅牢に運用するための基本法を定義する。

---

## 第1条：物理的境界 (The Cage)

AI アクターは、システムが提供する「檻 (Jail)」の外にあるリソースに直接アクセスしてはならない。

1.  **Dependency Injection**: アクターの `execute` メソッドは、引数として必ず `bastion::fs_guard::Jail` ハンドルを受け取らなければならない。
2.  **Access Control**: ファイル操作は、提供された `Jail` を介して取得した `SafePath` 上でのみ許可される。
3.  **Escalation**: Jail 外へのアクセス試行が検知された場合、アクターは即座に停止され、セキュリティイベントとして記録される。

---

## 第2条：通信プロトコル (The Contract)

アクター間のやり取りは、生テキストプロンプトではなく、厳格に定義された「契約（型）」に基づいて行われなければならない。

1.  **Type Safety**: 全てのリクエストとレスポンスは Rust の構造体として定義され、`serde` によるバリデーションを通過しなければならない。
2.  **No Hallucination**: AI のハルシネーション（嘘の形式）は、型変換（Deserialization）の段階で物理的に遮断する。
3.  **Traceability**: 全てのメッセージは `trace_id` を保持し、命令の発生源と伝搬経路を完全に追跡可能にする。

---

## 第3条：統治構造 (The Governance)

個々のアクターは失敗する可能性があることを前提とし、システム全体でその失敗を制御・修復しなければならない。

1.  **Supervision Tree**: アクターは `Supervisor` の監視下で実行される。アクターのパニックは Supervisor が捕捉する。
2.  **Restart Policy**: クラッシュ時、Supervisor は定義されたポリシー（即時再起動、待機、エスカレーション）に基づき、クリーンな環境でアクターを再生成する。
3.  **Self-Healing**: 重大なセキュリティ違反やリソース枯渇が検知された場合、Supervisor は都市全体（システム）への影響を防ぐためにアクターを隔離・終了させる。

---

## 第4条：生存維持 (Stability & Operations)

システムは、24/7 稼働に耐えうる健康状態を自己監視し、機密情報を適切に防衛しなければならない。

1.  **Health Monitoring**: プロセスは `HealthMonitor` を保持し、メモリ使用率やファイルディスクリプタを常時監視する。閾値を超えた場合は、統治機構（Supervisor）に通知しなければならない。
2.  **Secret Encapsulation**: API キー等の機密情報は `Secret<T>` 型でラップし、ログやデバッガへの意図しない露出を型システムレベルで防御しなければならない。
3.  **Graceful Exit**: 終了信号を受信した際は、進行中の全ての `Jail` 内タスクを安全に完了または中断し、ゾンビプロセスや一時ファイルを残さずに退出しなければならない。

---

## 第5条：自律的生産 (Autonomous Production)

システムは、複数のアクターを統轄し、目標に向かって効率的かつ安全にバッチ処理を遂行しなければならない。

1.  **Orchestration**: 高次元のタスク（動画量産等）は、複数のアクターを管理する「Orchestrator」によって段階的に実行されなければならない。
2.  **Pre-flight Health Checks**: バッチ処理の各ステップを開始する前に、Orchestrator は現在の `HealthMonitor` ステータスを確認し、資源の安全性（メモリ余裕等）を担保しなければならない。
3.  **Loop Integrity**: 量産ループ内での例外（単数アクターの失敗）は、生産ライン全体を停止させることなく、適切に捕捉・記録・リカバリされなければならない。

---

## 第6条：資源共有 (Resource Arbitration)

Mac mini M4 Pro 等の限られたハードウェア資源において、複数の重負荷アクターによる資源競合を防止し、安定した性能を担保しなければならない。

1.  **Single-Tenant Policy**: VRAM を大量に消費するタスク（LLM, TTS, ImageGen）は、同時に複数が実行されてはならない。
2.  **Resource Arbiter**: 重負荷アクターは、実行前に `ResourceArbiter` から資源占有権（Guard）を取得し、終了時に即座に解放しなければならない。
3.  **Explicit Memory Release**: モデルを使用するアクターは、タスク完了後、API （例：Ollama の `keep_alive: 0`）を通じて VRAM 上のモデルを明示的にアンロードし、ハードウェアを次のタスクに明け渡さなければならない。

---

## 第7条：放送品質 (Broadcast Grade Quality)

システムが生成する最終成果物は、視聴者の離脱を防ぎ、プラットフォームの品質基準を満たす「放送局グレード」でなければならない。

1.  **The Reaper (Process Lifecycle)**: 外部のサイドカープロセス（TTSサーバー等）は `SidecarManager` によって管理され、終了時は PGID を用いた「Graceful-then-Hard Kill」プロトコルを遵守しなければならない。
2.  **The Cameraman (Visual Smoothness)**: 静止画から動画を生成する際は、数学的なイージング関数を用いた Ken Burns エフェクトを適用し、機械的な直線移動（Jitter）を排除しなければならない。
3.  **The Sound Mixer (Audio Standards)**: 音声合成と BGM の合成には「Audio Ducking（サイドチェーン圧縮）」を適用し、全体の音圧は EBU R128 (-14 LUFS) 規格に準拠させなければならない。

---

## 第8条：外部監視 (The Watchtower)

システムは、遠隔地から安全に監視・制御可能でなければならず、その通信は「不信（Zero Trust）」に基づかなければならない。

1.  **Observer-Controller Pattern**: Discord Bot 等の外部インターフェースは直接ロジックを実行せず、UDS を介して Core に「リクエスト」を送り、Core がそれを検証・実行する。
2.  **Framed Communication**: 通信は `LengthDelimitedCodec` 等の明確なフレーム管理を行い、ネットワークの断片化や不正なパケットによるバッファオーバーフローを物理的に遮断しなければならない。
3.  **HITL (Human-In-The-Loop)**: 重大な決定（生成開始、再起動等）は、外部監視装置を通じて人間の承認を得る仕組みを提供し、完全自動化に伴う暴走リスクを軽減しなければならない。

---

## 第9条：自律進化 (The Forge)

システムは、自身の能力が不足している場合に、新しい「技能（Skill）」をコードとして自ら書き下ろし、それを安全なサンドボックス内で実行・統合する能力を保持する。

1.  **Isolated Forging**: 新しいスキルの生成とコンパイルは、ホストのファイルシステムや環境変数から完全に隔離された一時領域で行われなければならない。
2.  **Safety Compliance**: 生成されたコードは、PDK (Plugin Development Kit) の制約と「鉄の掟（SKILL_FORGE_PROMPT）」を遵守し、`build.rs` などのホスト側実行フックを含んではならない。
3.  **Strict Sandboxing**: 十二分なリソース制限（メモリ・CPU時間）を適用した WASM ランタイム上で実行し、意図しない無限ループやリソース枯渇からホストを防衛しなければならない。
4.  **Verification and Synthesis**: スキルの生出力は、最終的に「人格（SOUL）」を持つアクターによって検証・再構成（Synthesis）され、マスターに届けられなければならない。

---

## 付則：実装方針

-   **Core First**: 本法典のインターフェースは `libs/core` に定義し、具体的なインフラ実装（`libs/infrastructure`）と分離する。
-   **Strict Mode**: 本番環境においては `ENFORCE_GUARDRAIL=true` を常時適用し、法規違反を一切許容しない。
-   **Operational Dashboard**: 稼働状況（Health）は常に `api-server` や `watchtower` を通を通じて可視化され、人間の監視者が状況を即座に把握できるようにする。
-   **Evolutionary Ledger**: 自律進化によって追加された技能（WASM）の履歴は、いつ、どのような理由で追加されたかを完全に追跡可能な状態で記録されなければならない。

---

*最終更新: 2026-03-03*
*文書管理: Modular OpenClaw Security Team*
