# Security Policy
## セキュリティ・ポリシー (日本語併記)

### Supported Versions (サポート対象バージョン)

We provide security updates for the following versions:
現在、以下のバージョンのセキュリティ・アップデートを提供しています。

| Version | Supported          |
| ------- | ------------------ |
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

Aiome employs **Formal Verification** and **Design by Contract (DbC)** to ensure absolute isolation for autonomous agents. Our architecture guarantees safety mathematically, moving beyond traditional empirical testing:

1. **TLA+ Spec Verification**: The `AiomeQuarantineProtocol` (defining the isolation logic before a WASM skill becomes active) has been formally verified using the **TLC Model Checker**. We guarantee with algorithmic certainty that *no invalid or potentially harmful skill can bypass the sandbox to enter an active state*.
2. **Deterministic Tracer (Layer 3)**: Skills are dry-run in a completely deterministic, network-denied, memory-constrained WASM environment. Any resource violation or illegal syscall results in a definitive rejection.
3. **State Machine Contracts (Layer 4)**: The transition from `UnverifiedSkill` to `VerifiedSkill` is enforced at compile/runtime using Rust's TypeState patterns and DbC macros (`#[requires]`, `#[ensures]`), making security boundary breaches virtually impossible at the SDK level.

---
For technical details on our security architecture, see [docs/SECURITY_DESIGN.md](docs/SECURITY_DESIGN.md).
技術的な詳細については、[docs/SECURITY_DESIGN.md](docs/SECURITY_DESIGN.md) を参照してください。

*Maintenance: Aiome Security Team*
