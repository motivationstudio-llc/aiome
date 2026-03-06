# Aiome Quarantine Protocol - Trace Map

This document maps the abstract state transitions defined in the TLA+ specification (`AiomeQuarantineProtocol.tla`) to the concrete Rust implementation in the Aiome codebase.

## State Transitions

| TLA+ State Transition | Concrete Rust Code Path | Description |
| :--- | :--- | :--- |
| `Downloading` → `ManifestCheck` | `SkillForge::forge_skill` | A new skill is downloaded or generated. It then moves to the verification phase. |
| `ManifestCheck` → `DryRunQuarantine` | `UnverifiedSkill::verify` (Manifest OK) | The skill's metadata and requested capabilities are mathematically and statically checked. If within safe bounds, it enters quarantine. |
| `ManifestCheck` → `Violated`   | `UnverifiedSkill::verify` (Manifest Invalid) | The skill's manifest violates bounds (e.g., wildcard network requests). It is rejected. |
| `DryRunQuarantine` → `Active`  | `WasmSkillManager::dry_run_skill` (Success) | Simulated execution within the constrained Extism sandbox completes without resource violation or illegal syscalls. Skill is promoted to `VerifiedSkill`. |
| `DryRunQuarantine` → `Violated`| `WasmSkillManager::dry_run_skill` (Error) | Simulated execution fails due to OOM, timeout, or an unauthorized `.wasm` binding attempt. |
| `Active`                       | `WasmSkillManager::call_skill` | A fully verified skill is safely executed in the production sandbox. |

## Model-Based Testing (MBT) Strategy

The transition map above serves as the foundation for the MBT integration test (`libs/infrastructure/tests/mbt_quarantine.rs`). The test must execute logic matching these exact pathways:

1. **Happy Path**: `Downloading` → `ManifestCheck` → `DryRunQuarantine` → `Active` (Expected: `Ok(VerifiedSkill)`)
2. **Negative Path 1 (Bad Payload)**: `Downloading` $\rightarrow$ `ManifestCheck` $\rightarrow$ `Violated` (Expected: Rejected by DbC macro before dry-run)
3. **Negative Path 2 (Dry-Run Failure)**: `ManifestCheck` $\rightarrow$ `DryRunQuarantine` $\rightarrow$ `Violated` (Expected: Rejected due to Deterministic Tracer timeout or violation)
