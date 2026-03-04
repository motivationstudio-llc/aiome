# Contributing to Aiome

First of all, thank you for considering contributing! It's people like you that make Aiome better for everyone.

## Table of Contents
1. [Our Open-Core Model](#our-open-core-model)
2. [How Can I Contribute?](#how-can-i-contribute)
3. [Development Setup](#development-setup)
4. [Pull Request Process](#pull-request-process)
5. [Coding Standards & Architecture](#coding-standards--architecture)
6. [License & CLA](#license--cla)

---

## Our Open-Core Model

Aiome follows an **Open-Core** model:
- **Core (OSS)**: Under AGPL-3.0. Includes the framework, Karma system, and basic Watchtower features.
- **Pro/Enterprise**: Features for mass-scale GPU orchestration and advanced Skill Forge capabilities are proprietary.

## How Can I Contribute?

- **Bug Reports**: Open an issue with a clear description and steps to reproduce.
- **Feature Requests**: We love ideas! Please check if a similar request already exists.
- **Code**: Fork the repo, create a branch, and submit a PR.

## Development Setup

The project is built with **Rust**.

### Prerequisites
- **Rust**: 1.75+ (Stable)
- **Ollama**: For local LLM processing (Qwen2.5-Coder recommended).
- **ComfyUI**: Required for image/video generation tasks.
- **FFmpeg**: Required for media processing.

### Building
```bash
cargo build --workspace
```

### Testing
```bash
cargo test --workspace
```

## Pull Request Process

1. Fork the repository and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. Ensure the test suite passes.
4. Run license compliance check: `cargo deny check license`.
5. **Sign the CLA**: Your PR will only be merged once the CLA check passes.
6. Submit the PR!

## Coding Standards & Architecture

We follow a strict **Modular Workspace** architecture. Contributors MUST respect the following boundaries:

- **apps/api-server & shorts-factory**: Main entry points and DI containers.
- **libs/infrastructure**: Handle I/O (DB, Redis, External APIs).
- **libs/core**: Pure Domain Logic, Entities, and Interfaces. 
    - **CRITICAL**: `core` MUST NOT depend on `infrastructure` (Dependency Inversion Principle).
- **libs/shared**: Common utils and types. MUST NOT depend on any other layers.

### Iron Principles:
- **Result Type Mandatory**: No `unwrap()` or `expect()` outside of tests.
- **Type Safety**: Use NewType patterns and Enums for data flow.
- **Async First**: All I/O must be non-blocking using `tokio`.
- **Error Handling**: Use `anyhow` for apps and `thiserror` for library layers.

## License & CLA

By contributing, you agree that your contributions will be licensed under **AGPL-3.0** and you agree to the terms of our [CLA.md](./CLA.md).

---
*Happy coding!*
