---
title: Shortcut Service Boundary Refactor
type: refactor
status: active
date: 2026-04-16
---

# Shortcut Service Boundary Refactor Plan

## Overview

Refactor the desktop shortcut subsystem from a mixed runtime-plus-business-logic module into a layered backend service. The target shape keeps platform event tap and hotkey runtime details inside `shortcut/`, moves recording/cancel policy into `services/`, and leaves Tauri command/event adaptation in `commands/` and `lib.rs`.

This plan supersedes the architectural intent of the older handy-keys migration plan without rewriting history: the handy-keys migration already landed, but the resulting module still violates the current backend layer contract.

## Problem Frame

- **Current state:** `shortcut/manager.rs` owns macOS runtime lifecycle, desired/live registration state, trigger-to-recording policy, cancel policy, `AppState` inspection, and direct calls into audio commands.
- **Current state:** hotkey replacement logic is duplicated across startup, settings update, and hotkey capture completion, which makes it hard to reason about the single source of truth for shortcut state.
- **Current state:** `shortcut` emits Tauri events directly while also using internal channels, so domain events and UI adapter behavior are mixed together.
- **Desired state:** `shortcut/` becomes an input runtime and typed-intent producer, not the owner of recording business decisions.
- **Desired state:** `services/shortcut.rs` owns trigger policy, capture/replace workflow rules, and other shortcut-domain use cases in a headless-friendly way.
- **Desired state:** commands and setup code invoke a small service surface instead of remembering unregister/register order or manager internals.

## Scope Boundaries

### In scope

- Introduce a backend shortcut service module under `src/services/`.
- Extract pure policy decisions from `shortcut/manager.rs` into the service layer.
- Extract hotkey replacement workflow into the service layer where practical without changing frontend behavior.
- Keep `ShortcutManager::start()` as the only application bootstrap entry for runtime startup.
- Add or update focused tests that lock the service contract before broader refactors.

### Out of scope

- Replacing `handy-keys` or changing the cross-platform runtime backend.
- Rewriting the entire shortcut subsystem into traits and adapters in one pass.
- Frontend UI redesign or IPC signature changes in `src/lib/tauri.ts`.
- New shortcut features such as multi-binding profiles or per-window scopes.

## Target Architecture

### Target layers

- `shortcut/`
  - Owns platform runtime, permission/probe lifecycle, live OS registrations, capture backends.
  - Emits typed shortcut intents or low-level runtime outcomes.
- `services/shortcut.rs`
  - Owns recording trigger policy, cancel owner policy, hotkey replacement workflow, capture completion rules.
  - Depends on `state/` and `shortcut::ShortcutManager`, but not on Tauri handles or frontend events.
- `commands/hotkey.rs` and `lib.rs`
  - Thin adapters that call service functions and perform Tauri event emission or command dispatch.

### Desired end state

- Primary shortcut press/release is translated into a typed action first, then executed by an adapter.
- Cancel shortcut owner tracking is decided by service functions, not embedded in manager event handlers.
- Hotkey replacement logic lives in one backend service path instead of being duplicated across capture completion and settings mutation.
- `shortcut/manager.rs` no longer imports or decides recording workflow behavior directly.

## Implementation Units

### Unit 1: Extract shortcut policy service

**Goal**

Create `src/services/shortcut.rs` containing pure policy helpers for primary trigger actions, cancel owner tracking, and hotkey replacement workflow.

**Files**

- Add: `apps/desktop/src-tauri/src/services/shortcut.rs`
- Modify: `apps/desktop/src-tauri/src/services/mod.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Modify: `apps/desktop/src-tauri/src/commands/hotkey.rs`

**Approach**

- Add typed policy enums for primary trigger actions and cancel-owner decisions.
- Move the existing pure decision logic from `manager.rs` into the new service.
- Add a service helper for applying the primary hotkey swap sequence in one place.
- Keep the actual side effects in adapters for now: manager still calls audio commands, and hotkey command still emits frontend events, but both stop owning the policy logic.

**Verification**

- `cargo test --lib services::shortcut::tests`
- `cargo test shortcut::manager --lib`
- `cargo test commands::hotkey --lib`

### Unit 2: Move trigger execution out of manager

**Goal**

Stop `shortcut/manager.rs` from directly importing recording workflow behavior.

**Files**

- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Add/Modify: shortcut service adapters under `src/services/shortcut.rs` or a dedicated adapter helper
- Modify: `apps/desktop/src-tauri/src/lib.rs` if a background event bridge is needed

**Approach**

- Introduce a typed event/intent channel from manager to the application layer.
- Have an adapter consume intents and invoke recording/cancel commands.
- Preserve current behavior and event names while removing business rules from the runtime loop.

**Verification**

- Focused unit tests for intent mapping
- Integration verification that primary and cancel shortcuts still drive recording correctly

### Unit 3: Centralize backend hotkey mutation workflow

**Goal**

Make startup, settings hotkey changes, and capture completion use one backend hotkey mutation path.

**Files**

- Modify: `apps/desktop/src-tauri/src/commands/settings/mod.rs`
- Modify: `apps/desktop/src-tauri/src/commands/hotkey.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify or extend: `apps/desktop/src-tauri/src/services/shortcut.rs`

**Approach**

- Introduce a single service entry for "apply primary hotkey".
- Remove duplicated unregister/register sequencing from command/setup code.
- Keep persistence and frontend event emission in command/setup adapters.

**Verification**

- Settings-focused tests for hotkey update behavior
- Hotkey capture completion tests
- `cargo test --lib`

### Unit 4: Split runtime backend from capture backend

**Goal**

Prepare `shortcut/` for platform abstraction and future backend substitution.

**Files**

- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/listener.rs`
- Add: backend-oriented modules under `apps/desktop/src-tauri/src/shortcut/`

**Approach**

- Separate live registration runtime from capture runtime.
- Consolidate macOS-only helpers such as `FnEmojiBlocker` and probe logic around backend responsibilities rather than business entry points.

**Verification**

- Targeted unit tests for backend state transitions
- `cargo check`

### Unit 5: Move UI emission to explicit adapters

**Goal**

Separate domain/runtime events from frontend-facing Tauri events.

**Files**

- Modify: `apps/desktop/src-tauri/src/events/mod.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/commands/hotkey.rs`

**Approach**

- Use internal typed events first, then map to Tauri emit calls in adapters.
- Remove direct `app.emit(...)` calls from core shortcut runtime where possible.

**Verification**

- Event helper tests
- End-to-end smoke verification for shortcut-triggered flows

## System-Wide Impact

- Recording start/stop/cancel policy becomes testable without booting the full shortcut runtime.
- Settings and capture workflows stop carrying duplicated hotkey replacement logic.
- The shortcut subsystem becomes more compatible with the project rule that services must not depend on commands or Tauri side effects.
- Future platform fixes should be limited to backend runtime modules instead of leaking into service or command layers.

## Risks & Dependencies

- There is no dedicated versioned feature spec for shortcut architecture, so this refactor relies on `context/spec/hotkey.md`, current tests, and architecture rules as the behavioral baseline.
- `ShortcutManager` currently accesses internal state directly for cancel-owner bookkeeping; extracting too much in one pass could destabilize cancellation behavior.
- Startup and settings hotkey application order already exists in production paths; consolidating them later must preserve current persistence and event emission semantics.

## Verification Evidence

- Planning inputs reviewed:
  - `context/spec/hotkey.md`
  - `context/architecture/layers.md`
  - `context/architecture/data-flow.md`
  - `context/plans/active/2026-04-11-001-refactor-shortcut-to-handy-keys-plan.md`
- Unit 1 is the active execution slice for this session.
