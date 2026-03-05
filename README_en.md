<div align="right">
  <a href="README.md">日本語</a> | <strong>English</strong>
</div>

<p align="center">
  <img src="docs/assets/logo.png" alt="Aiome Logo" width="300">
</p>

<h1 align="center">Aiome</h1>
<p align="center">
  <strong>The Autonomous AI Operating System for Self-Evolving Agents</strong><br>
  <em>Build AI that Learns, Defends, and Evolves — Autonomously.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/License-AGPL--3.0-red.svg" alt="License: AGPL-3.0">
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange.svg" alt="Rust 1.75+">
  <img src="https://img.shields.io/badge/PRs-Welcome-brightgreen.svg" alt="PRs Welcome">
</p>

---

## 🌌 What is Aiome? (Concept)

Aiome is more than just a task execution tool. It is a **next-generation autonomous AI operating system** that accumulates "lessons" (Karma) with each execution, protects itself from threats (Immune System), shares intelligence with other nodes (Federation), and forms its own unique "personality" (SOUL).

Video generation (e.g., Project-Boring) is merely one of the "skills" (modules) running on top of this powerful OS.

### 🛡️ 4 Core Pillars

1.  **Self-Evolution (Karma)**: A learning capability that accumulates both failures and successes in SQLite to ensure the same mistakes are not repeated.
2.  **Self-Defense (Immune System)**: An immune system that detects malicious outputs or infinite loops and autonomously cuts off and repairs circuits.
3.  **Swarm Intelligence (Federation)**: Instantly synchronizes "lessons learned" across global Aiome nodes via the Samsara Hub.
4.  **Personality (SOUL Architecture)**: An identity simulated through dialogue with the user, transforming the AI from a mere tool into a true "partner."

---

## 🏗️ Architecture (Open-Core Strategy)

<table align="right">
  <tr>
    <td align="center">
      <img src="docs/assets/actor.png" width="220"><br>
      <b>【Actor】</b>
    </td>
  </tr>
</table>

This project adopts an **Open-Core Model** to foster a healthy ecosystem.

### 🟢 Aiome Core (OSS Version - AGPL-3.0)
The foundational Karma scheme, Immune defense, Federation synchronization, and basic SOUL engine are provided as open source.

### 🔴 Aiome Pro / Enterprise (Commercial License)
Advanced parallel processing (GPU Cluster), the high-performance execution engine (Advanced Skill Forge), and managed Hub features for enterprises are provided under a commercial license.

```text
apps/shorts-factory  ← Main Binary (The Body / Open & Pro)
      ↓
libs/core            ← Domain Logic (Open)
      ↓
libs/infrastructure  ← I/O Impl (ComfyUI, SQLite / Open)
      ↓
libs/shared          ← Common Types, Guardrails (Open)
```

---

## ✨ Capabilities

By deploying Aiome, you can instantly build autonomous workflows like the following:

- 🧠 **Autonomous Loop**: Fully automates everything from planning to generation by monitoring trends 24/7 without user intervention.
- 🎬 **Media Generation Ecosystem**: Seamless integration with ComfyUI (Image/Video generation) and FFmpeg (Audio/Video editing).
- 🗣️ **Discord Interaction Interface**: Issue commands and converse with the system using natural language via a personified gateway called the "Watchtower."
- 🛡️ **Robust Error Self-Healing**: Detects execution errors and LLM hallucinations, autonomously modifies its configuration within the sandbox, and retries.

---

## 🧩 Extensibility (Skill Ecosystem)

Aiome's true power lies in its **extreme extensibility leveraging WASM (WebAssembly)**.

- **Safe Sandbox**: Additional features (skills) run in an isolated WASM environment, ensuring they do not compromise the safety of the core system.
- **Auto-Forging**: Features a "Skill Forge" (Pro/Advanced feature) where the AI programs, implements, and deploys necessary functionalities on the fly.
- **Community Shared**: Custom skills you develop will eventually be shareable with other nodes via the SAMSARA Hub.

---

## 🛠️ Technical Stack

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white)
![FFmpeg](https://img.shields.io/badge/FFmpeg-%232B65EC.svg?style=for-the-badge&logo=ffmpeg&logoColor=white)

| Component | Technology | Role |
|---|---|---|
| **Core Engine** | Rust | Fast, memory-safe orchestrator |
| **LLM Backend** | Ollama (Qwen, Mistral, etc.) | Thought circuits via local inference |
| **Media Engine** | ComfyUI / FFmpeg | Transmutation of images, videos, and audio |
| **Storage** | SQLite | Persistence of memories (Karma) and config |
| **Expansion** | WebAssembly (Wasm) | Safe and portable skill execution environment |

---

## 🛰️ Execution Components

<table align="right">
  <tr>
    <td align="center">
      <img src="docs/assets/watchtower.png" width="220"><br>
      <b>【WATCHTOWER】</b>
    </td>
  </tr>
</table>

### 1. Watchtower — The Manifestation of SOUL
Watchtower is the gateway for a master to interact with Aiome's "Personality." Through Discord, it reports system status, awaits instructions, and offers autonomous suggestions.

- **Details**: [docs/WATCHTOWER_USER_GUIDE.md](docs/WATCHTOWER_USER_GUIDE.md) *(JP)*
- **Personality Manifest**: [WATCHTOWER_MANIFEST.md](WATCHTOWER_MANIFEST.md) 🐾

### 2. Factory / Skills (Skills & Modules)
Specific applications running on Aiome Core.

- **Shorts Factory**: Fully automated video mass-production for YouTube Shorts.

---

## 🚀 Quick Start

### 1. Prerequisites
Ensure the following backend services are running:
- **Ollama**: `ollama serve` (Recommended model: `qwen2.5-coder`)
- **ComfyUI**: Web UI (`http://localhost:8188`)

### 2. Setup & Run
```bash
# 1. Clone the repository
git clone https://github.com/motivationstudio-llc/aiome
cd aiome

# 2. Configure environment variables (API keys, etc.)
cp .env.example .env

# 3. Start Aiome Core (Samsara Protocol)
cargo run -p shorts-factory -- serve
```

> **Note**: To use the Discord integration (Watchtower), run `cargo run -p watchtower` in a separate terminal.

#### 🔑 Key Environment Variables (.env)
- `DISCORD_TOKEN`: For Watchtower.
- `OLLAMA_BASE_URL`: For LLM connections (Default: http://localhost:11434).
- `COMFYUI_URL`: For the generative engine (Default: http://localhost:8188).

---

## 📚 Documentation (Mostly JP)

- **[AI Architecture Law](docs/ARCHITECTURE_LAW.md)**: Foundational principles for intellectual integrity and safety.
- **[Operations Guide](docs/OPERATIONS_MANUAL.md)**: Detailed setup and operational procedures.
- **[Evolution Strategy](docs/EVOLUTION_STRATEGY.md)**: Design philosophy of self-evolution and rearing systems.
- **[Soul Customization](docs/CUSTOMIZING_SOUL.md)**: Adjusting the AI's personality and reactions.
- **[Security Design](docs/SECURITY_DESIGN.md)**: Deep dive into the multi-layered defense.

---

## 🤝 Contributing

- **[Contributing Guide](CONTRIBUTING.md)**: Rules for participating in development.
- **[CLA](CLA.md)**: Contributor License Agreement.
- **[Code of Conduct](CODE_OF_CONDUCT.md)**: Behavioral standards.
- **[Security Reporting](SECURITY.md)**: Contact for security issues.

---

## 🛡️ License

**Aiome Core** is provided under **AGPL-3.0**. For commercial use, contact [motivationstudio,LLC](https://github.com/motivationstudio-llc/aiome).

*Built by [motivationstudio,LLC](https://github.com/motivationstudio-llc) — Powering the Future of AI Autonomy.*
