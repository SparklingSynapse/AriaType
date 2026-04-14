---
title: "fix: Centralize startup permission logging in backend permission domain"
type: fix
status: completed
date: 2026-04-14
---

# fix: Centralize Startup Permission Logging In Backend Permission Domain

## Overview

Add a backend-owned permission domain that can produce a typed permission snapshot and emit startup logs at the earliest safe point in app boot. The goal is to replace ad-hoc permission checks in `lib.rs` with one reusable entry point that scales as the desktop app adds more permission-gated capabilities.

## Problem Frame

Current startup permission logging is fragmented:
- `lib.rs` checks accessibility directly and logs a one-off status.
- `lib.rs` also performs custom microphone request/status handling with its own logging.
- `commands::permissions` mixes domain logic, platform checks, and Tauri IPC wrappers in one module.

This makes startup behavior hard to reason about, encourages permission logic to sprawl across the app entrypoint, and raises regression risk whenever a new permission is added.

## Scope Boundaries

- **In scope**: backend permission modeling, startup snapshot logging, refactor of permission IPC wrappers, minimal doc/plan updates
- **Out of scope**: frontend permission UX, adding new OS capabilities, changing hotkey recovery behavior, changing existing permission prompts beyond necessary call-site cleanup

## Implementation Units

- [x] **Unit 1: Introduce a backend permission domain**

  Create `src/permissions/` with:
  - typed `PermissionKind`
  - typed `PermissionStatus`
  - `PermissionSnapshot`
  - `PermissionDefinition`
  - platform provider abstraction

  Verification:
  - unit tests for string round-trip and snapshot field lookup

- [x] **Unit 2: Centralize startup logging**

  Add a single startup entry point such as `report_startup_permission_snapshot()` that:
  - reads all known permissions
  - emits one snapshot summary log
  - emits one structured per-permission log
  - keeps the log format aligned with `context/spec/logs.md`

  Verification:
  - compile check of the touched Rust files
  - manual inspection of log field names in code

- [x] **Unit 3: Reduce command layer to IPC wrappers**

  Update `commands::permissions` so Tauri commands delegate to the permission domain instead of owning platform logic directly.

  Verification:
  - `check_permission` and `apply_permission` signatures remain compatible with frontend IPC

- [x] **Unit 4: Replace ad-hoc startup checks in `lib.rs`**

  Remove direct permission inspection from `lib.rs` and call the centralized startup logger at app setup.

  Verification:
  - startup path still performs microphone first-launch flow
  - startup path no longer duplicates accessibility status logging

## System-Wide Impact

- `lib.rs` gets simpler and loses permission-specific branching.
- Future permission additions should only need new domain definitions and platform provider support.
- Frontend keeps using stable IPC contracts while backend ownership becomes clearer.

## Risks & Dependencies

- Startup logging must remain read-only. Logging should not accidentally trigger permission prompts.
- The existing microphone first-launch request flow must keep its current behavior while being explained by the new domain model.
- Build verification may still be partially blocked by external `sherpa-onnx-sys` download failures.

## Verification Evidence

- `cargo test permissions::tests --lib` from `apps/desktop/src-tauri` — passed
- `GetDiagnostics` on touched Rust files — no Rust diagnostics introduced; only existing cSpell info hints remain
- `rustfmt --edition 2021 src/lib.rs src/commands/permissions/mod.rs src/permissions/mod.rs src/permissions/macos.rs src/permissions/windows.rs` — passed
