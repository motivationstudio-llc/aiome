# 🌌 Documentation Hub - Aiome
Welcome to the Aiome project documentation. This wiki is automatically generated.

## 🏗️ Architecture & Constitution

- **[Lex AI Constitution](./ARCHITECTURE_LAW.md)**: AI 都市建築基準法。アクターの境界、契約、統治を規定。
- **Apps**: 
    - `api-server`: Main management hub. Orchestrates agent chat, skill execution, and security monitoring.
    - `watchtower`: Monitoring and gateway actor.
    - `samsara-hub`: Central validator for federation network.
- **Libs**: 
    - `core`: 基本コントラクト、トレイト定義。
    - `infrastructure`: I/O（DB、API、ツール）の具体的実装。
    - `shared`: ユーティリティ、共通型定義。

## 🛡️ Iron Principles

- **Result Type Mandatory**: `unwrap()` and `expect()` are forbidden outside tests.
- **Lex AI Compliance**: Actors MUST use `Jail`, `Contracts`, and run under a `Supervisor`.
- **Resource Discipline**: Every component must be `HealthMonitor` friendly and use `Secret<T>` for sensitive data.
- **Fail-Safe Design**: Default to `DENY`. Security violations trigger immediate isolation.
- **Async/Await**: Powered by `tokio` for high-performance non-blocking operations.
