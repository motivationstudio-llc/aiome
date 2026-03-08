# Security Policy
## セキュリティ・ポリシー (日本語併記)

### Supported Versions (サポート対象バージョン)

We provide security updates for the following versions:
現在、以下のバージョンのセキュリティ・アップデートを提供しています。

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

### Reporting a Vulnerability (脆弱性の報告方法)

**Please do not open a public issue for security vulnerabilities.**
**脆弱性に関する公開Issueを作成しないでください。**

If you discover a security vulnerability, please report it through the following channels:
脆弱性を発見された場合は、以下のチャンネルを通じて報告してください。

1. **GitHub Security Advisories (Primary / 推奨)**: Use the "Report a vulnerability" button on the Security tab of our repository. This is the fastest and most secure way to reach the maintainers. (リポジトリのセキュリティタブにある「Report a vulnerability」ボタンを使用してください。これが最も速く安全な報告方法です。)
2. **Email (Secondary / 補助)**: You can also contact us at **project.aiome@gmail.com** if you prefer or cannot use GitHub's advisory system. (GitHubのシステムを利用できない場合は、メールでも受け付けています。)

---

### Our Commitment (私たちの公諾)
- We will acknowledge receipt of your report within **72 hours**. (報告受領から72時間以内に一次回答を行います。)
- We will provide a fix or mitigation plan as soon as possible. (可能な限り迅速に修正または緩和策を提供します。)
- We will keep you informed of our progress. (進捗状況を定期的にお知らせします。)

### Responsible Disclosure (責任ある開示)
We ask that you follow responsible disclosure principles:
責任ある開示の原則に従うようお願いします：
- Give us a reasonable amount of time to fix the issue before making it public. (公開前に修正のための合理的な時間を確保してください。)
- Do not exploit the vulnerability beyond what is necessary to prove its existence. (脆弱性の証明に必要な範囲を超えて悪用しないでください。)
- Do not access or modify data belonging to other users. (他のユーザーのデータにアクセスしたり改変したりしないでください。)

---

### Mathematical Security Guarantees (数学的セキュリティ保証)

Aiome employs **Formal Verification**, **Model-Based Testing (MBT)**, and **Design by Contract (DbC/TypeState)** to assure absolute isolation and logical consistency for autonomous agents. Our architecture guarantees safety mathematically, moving far beyond traditional empirical testing:

1. **TLA+ Spec Verification**: The `AiomeQuarantineProtocol` (WASM sandbox isolation logic), `AiomeContextEngine` (FSM of the high-level agent cognition), and the `SamsaraKarmaProtocol` (Hash-chain-based knowledge federation) are formally verified using the **TLC Model Checker**. We guarantee with algorithmic certainty that liveness properties hold (no deadlocks) and safety invariants (e.g., `CompactMutex`) are never breached.
2. **NAPI Hybrid Boundary Defense**: Our architecture separates high-level Plugin/UI logic (Node.js) from the low-level, verified core (Rust) using a **NAPI-rs bridge (Sentinel)**. This ensures that even if the Node.js layer is compromised, the Rust core's invariants (enforced by TLA+ models and TypeState) protect critical state (Karma/Immune rules) and enforce strict path/command blacklists via `immune_check_tool`.
3. **Model-Based Testing (MBT) at CI/CD**: Formal TLA+ state transitions are mapped to concrete execution paths via `TRACE_MAP`. Our integration tests automatically verify these deterministic path executions on every PR, ensuring the implementation perfectly respects the mathematical model.
4. **TypeState Compile-Time Enforcement (Layer 3½)**: The transition from `UnverifiedSkill` to `VerifiedSkill` is enforced at compile time. Functions executing skills (`call_skill`) demand the `VerifiedSkill` type, making security boundary breaches virtually impossible at the SDK level (the compiler simply rejects them).
5. **Deterministic Tracer (Layer 3)**: Skills are dry-run in a completely deterministic, network-denied, memory-constrained WASM environment. Any resource violation or illegal syscall results in a definitive rejection.
6. **OS-Native Build Isolation (The Forge Protocol)**: When the agent autonomously generates and compiles Rust code (`cargo build`), the compilation process itself is strictly isolated via OS-native sandboxing (e.g., Mac's `sandbox-exec` or Linux `bwrap`). This isolates the host operating system from any build-time supply chain attacks (e.g., malicious `build.rs` macros).
7. **Core State Actor Concurrency**: The SQLite state engine acts as a single centralized messaging `Tokio MPSC Channel` (Actor Pattern). This mathematically prevents database lock errors and race conditions that could otherwise crash the NAPI interface layer during massive Swarm Sync traffic.


---
For technical details on our security architecture, see [docs/SECURITY_DESIGN.md](docs/SECURITY_DESIGN.md).
技術的な詳細については、[docs/SECURITY_DESIGN.md](docs/SECURITY_DESIGN.md) を参照してください。

*Maintenance: Aiome Security Team*
