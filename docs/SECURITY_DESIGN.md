# Aiome Security Design Doctrine

> This document defines the security architecture for Aiome. It records the rationale for design decisions and the responsibilities of each defense layer.

## 1. Core Principle: Zero-Trust for LLM

Unlike traditional agent frameworks that grant execution privileges to the LLM, Aiome operates on a **Zero-Trust** basis: **The LLM is restricted to "deliberation" while all execution is strictly managed by Rust-enforced guardrails.**

```
Traditional:  [LLM] → Arbitrary Code Execution → 💀 Risk of Hallucination/Malice
Aiome:        [LLM] → Rust Validation Layer → Whitelisted Tool Execution → ✅
```

## 2. Threat Model

### 2.1 Out-of-Scope Threats
- **DDoS (Internal)**: Most services are bound to `localhost` and are not exposed to the internet.
- **MITM**: Local inter-process communication (LDC/UDS) is used between trusted components.

### 2.2 Addressed Threats

| # | Threat | Vector | Severity | Mitigation Layer |
|---|---|---|---|---|
| 1 | Prompt Injection | User Input → LLM | 🔴 High | Input Guardrails |
| 2 | LLM Output Hallucination | LLM → Tool Execution | 🔴 High | OutputValidator + Formal Schema |
| 3 | Secret Leakage | API Keys in Memory | 🔴 High | **Abyss Vault (Key Proxy) + mlockall / zeroize** |
| 4 | Supply Chain Vulnerability | Dependencies | 🟡 Mid | `cargo audit` + Sentinel |
| 5 | Resource Exhaustion | Infinite Loop / Spams | 🟡 Mid | Rate Limiting + WASM Timeout & Circuit Breaker |
| 6 | **Karma Poisoning** | **Malicious Federation Sync** | 🔴 High | **Bearer Auth + Node Reputation System** |

## 3. Defense Architecture

### Layer 1: Guardrails (Input Validation)
- Detects prompt injections and command injections.
- Sanitizes control characters and enforces length limits.

### Layer 2: SecurityPolicy (Execution Control)
- **Whitelisting**: Only registered tools in the `ToolRegistry` can be executed.
- **Sandboxing**: Filesystem access is restricted via `PathSandbox`. WASM execution is strictly walled off from wildcard host access.
- **Abyss Vault**: ALL LLM and remote API calls are routed through an isolated Key Proxy process utilizing `mlockall` and exact endpoint routing to prevent SSRF and memory leakage.

### Layer 3: Audit Log & Hash Chains
- Every tool invocation and systemic decision is logged for post-hoc analysis.
- **Hash Chains**: All operational logs in SQLite are cryptographically linked using SHA-256 hash chains, enabling immediate detection of deletion or tampering efforts.

### Layer 4: Build Isolation & Formal TDD Forge (S-Rank Defense)
- **OS-Native Sandbox**: Autonomous compilation (`cargo build`) executed by the agent is forcibly containerized using OS-native guardrails (`sandbox-exec` / `bwrap`) to prevent supply chain attacks during the Forge process.
- **Fail-Forward Training**: Instead of terminating agents when code fails to compile, the system employs TDD-based reinforcement loops without permanent Karma penalties, allowing self-healing code generation.
- **Core State Actor**: Safe integration with Node.js NAPI layer uses a strictly serial MPSC Channel Actor Model, preventing async/sync deadlock scenarios inherently.

## 4. Operational Safety Layers

- **OutputValidator**: Automatically retries parsing when LLM returns invalid JSON schema.
- **PathSandbox**: Prevents directory traversal by enforcing canonical path prefix checks.
- **ZombieKiller**: Monitors and terminates hung external processes/subprocesses.
- **Karma Federation**: Synchronizes "learned lessons" across nodes using signed and authenticated payloads.

## 5. Comparison with Traditional Systems

| Criteria | Existing Frameworks | Aiome |
|---|---|---|
| LLM Privileges | Full Access | Whitelisted Only |
| Plugin Loading | Dynamic/Remote | Compile-time / WASM Sandbox |
| Memory Safety | GC-based (Python/JS) | Ownership-based (Rust) |
| Validation | Middleware Dependent | Hardened Core Implementation |

---
*Last Mutated: 2026-03-05*
*Managed by: Aiome Sovereign Task Force*
