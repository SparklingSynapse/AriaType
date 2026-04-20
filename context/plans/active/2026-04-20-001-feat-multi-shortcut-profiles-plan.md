---
title: Multi-Shortcut Profiles Implementation
type: feat
status: active
date: 2026-04-20
origin: context/feat/multi-shortcut/1.0.0/prd/erd.md
depends_on: context/plans/active/2026-04-16-002-refactor-shortcut-service-boundaries-plan.md
---

# Multi-Shortcut Profiles Implementation Plan

## Overview

Implement support for multiple keyboard shortcuts, each bound to a distinct action. Profiles store in settings, dispatch via enum match, and override resolution happens in `PreparedRecordingStart`.

## Problem Frame

**Current state**: Single `hotkey` field in settings, one registered hotkey, all recordings use global STT/Polish settings.

**Desired state**: `shortcut_profiles` array in settings, multiple registered hotkeys, each profile can override polish provider/model. Default profile preserves backward compatibility.

## Requirements Trace

From `context/feat/multi-shortcut/1.0.0/prd/erd.md`:

- R1. Default profile preserved after migration (AC-1)
- R2. Multiple profiles with different hotkeys and actions (AC-2)
- R3. Polish override uses specified provider/model (AC-3)
- R4. No override = global behavior (AC-4)
- R5. Hotkey conflict detection (AC-5)
- R6. Default profile undeletable (AC-6)
- R7. Single session invariant (AC-7)
- R8. Override fallback safety (AC-8)
- R9. Headless compatibility (AC-9)
- R10. Extensibility: new action variant = additive changes only (AC-12)

## Scope Boundaries

### In scope

- `ShortcutProfile` and `ShortcutAction` types
- Settings migration from `hotkey` to `shortcut_profiles`
- Multi-profile registration in ShortcutManager
- Profile CRUD IPC commands
- Polish override resolution in `PreparedRecordingStart`
- Frontend IPC and SettingsContext update

### Out of scope

- STT override (not requested)
- Language override (not requested)
- New action kinds beyond Record (future work)
- Frontend profile management UI (can be separate plan)
- MeetingRecord action (future work)

## Context & Research

### Relevant Code and Patterns

| Area | Pattern Source | Key Insight |
|------|----------------|-------------|
| Settings field addition | `commands/settings/mod.rs:70-124` | Use `#[serde(default)]` for graceful migration |
| Settings migration | `commands/settings/mod.rs:231-302` | `migrate_cloud_settings()` pattern: parse JSON, transform, save |
| PreparedRecordingStart | `services/recording_lifecycle.rs:5-10` | Captures config at preparation time; async task uses captured values |
| Hotkey registration | `shortcut/manager.rs:137-153` | Command queue pattern: `pending_commands` → background thread |
| Service pure functions | `services/shortcut.rs:36-62` | Context → Decision enum pattern |
| IPC commands | `commands/hotkey.rs` | `#[tauri::command]`, `Result<T, String>`, state via `try_state` |
| Event payload | `events/mod.rs` | Derive `Serialize`, emit via `app.emit()` |

### Dependency

Requires **Shortcut Service Boundary Refactor** (`2026-04-16-002`) Unit 1-2 complete. The service layer in `services/shortcut.rs` must exist before multi-profile dispatch logic can live there.

If refactor not complete: implement Units 1-3 of this plan first (types, settings, manager), then pause for refactor Units 1-2, then continue with this plan Units 4-7.

## Key Technical Decisions

Reference `context/feat/multi-shortcut/1.0.0/prd/erd.md` ADR sections:

- ADR-1: Enum variants per action (not composable overrides)
- ADR-2: Resolve into PreparedRecordingStart (not session-state)
- ADR-3: Profiles reference existing cloud configs
- ADR-4: Default profile undeletable
- ADR-5: Global recording mode
- ADR-6: No ShortcutConfig wrapper

## Implementation Units

### Unit 1: Add ShortcutProfile and ShortcutAction types

**Goal:** Define the core types for profiles and actions with serialization support.

**Requirements:** R10 (extensibility via enum), R1 (default profile format)

**Dependencies:** None

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/profile_types.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/mod.rs` (export module)

**Approach:**
- `ShortcutProfile` struct with `id`, `label`, `hotkey`, `action`
- `ShortcutAction` enum with `Record { polish_provider, polish_model }`
- All derive `Debug, Clone, PartialEq, Serialize, Deserialize`
- `#[serde(rename_all = "PascalCase")]` for enum serialization to match spec JSON format
- Default impl for `ShortcutProfile`: `id = "default"`, `hotkey = "Shift+Space"`

**Patterns to follow:**
- `shortcut/types.rs` for struct definition style
- Serde attributes from `commands/settings/mod.rs` field definitions

**Test scenarios:**
- Happy path: Serialize `ShortcutProfile` to JSON, deserialize back, fields match
- Happy path: `ShortcutAction::Record {}` serializes to `{ "Record": {} }`
- Edge case: `ShortcutAction::Record { polish_provider: Some("anthropic"), ... }` serializes with nested fields
- Error path: Deserialize invalid JSON → error

**Verification:**
- `cargo test --lib shortcut::profile_types`
- `cargo check`

---

### Unit 2: Settings migration from hotkey to shortcut_profiles

**Goal:** Replace `hotkey` field with `shortcut_profiles`, preserve existing hotkey behavior.

**Requirements:** R1 (default profile preserved), R6 (default exists), R4 (no override = global)

**Dependencies:** Unit 1 (types)

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/settings/mod.rs`
  - Add `shortcut_profiles: Vec<ShortcutProfile>` to `AppSettings`
  - Remove `hotkey: String` field (or keep for migration detection)
  - Add `#[serde(default)]` for backward compat
  - Add migration function `migrate_hotkey_to_profiles()`
- Modify: `apps/desktop/src-tauri/src/services/mod.rs` (if migration lives there)

**Approach:**
- Add `shortcut_profiles` field with `#[serde(default)]`
- Keep `hotkey` field temporarily for migration detection (remove after migration runs)
- Migration function in `load_settings_from_disk()`:
  - If `hotkey` exists and `shortcut_profiles` empty or missing: create default profile from `hotkey`
  - If `shortcut_profiles` has no `id = "default"`: insert default from hotkey fallback
  - Remove `hotkey` from JSON after migration
  - Save migrated settings
- `Default` for `shortcut_profiles`: single default profile with `Shift+Space`

**Patterns to follow:**
- `migrate_cloud_settings()` (lines 231-302) for migration structure
- `#[serde(default)]` pattern for field addition

**Test scenarios:**
- Happy path: Old settings `{ "hotkey": "Cmd+Space" }` → migrated to `shortcut_profiles: [{ id: "default", hotkey: "Cmd+Space", action: { Record: {} } }]`
- Happy path: Already migrated settings → no changes
- Edge case: Missing both `hotkey` and `shortcut_profiles` → default profile created
- Edge case: `shortcut_profiles` exists but no "default" → default inserted from hotkey fallback

**Verification:**
- `cargo test --lib commands::settings::migration`
- Manual: Start app with old settings file → verify migration

---

### Unit 3: ShortcutManager multi-profile registration

**Goal:** Replace single `current_id` with `registered_ids: HashMap<String, HotkeyId>` for multiple profiles.

**Requirements:** R2 (multiple profiles), R5 (hotkey conflict detection), R7 (single session), R9 (headless)

**Dependencies:** Unit 1 (types), Unit 2 (settings provides profiles)

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
  - Replace `current_id: Mutex<Option<HotkeyId>>` with `registered_ids: Mutex<HashMap<String, HotkeyId>>`
  - Extend `ShortcutCommand` with `RegisterProfile { profile: ShortcutProfile }`, `UnregisterProfile { id: String }`
  - Update `process_pending_commands()` for new commands
- Modify: `apps/desktop/src-tauri/src/shortcut/types.rs` (extend commands)

**Approach:**
- `ManagerState` field: `registered_ids: Mutex<HashMap<profile_id, HotkeyId>>`
- Registration command: `RegisterProfile { profile }` — unregister old hotkey for this profile_id if exists, validate hotkey not in use by other profile, register new, store in map
- Unregistration command: `UnregisterProfile { id }` — lookup hotkey_id, unregister, remove from map
- Hotkey conflict: before registering, check `registered_ids` values for hotkey string collision; if found, return error
- Startup: iterate `shortcut_profiles`, send `RegisterProfile` for each
- `handle_hotkey_event`: lookup `event.id` in `registered_ids.values()`, find matching `profile_id`, emit event with `profile_id`

**Patterns to follow:**
- Existing `register_primary()` command queue pattern
- `process_pending_commands()` structure

**Test scenarios:**
- Happy path: Register two profiles with different hotkeys → both registered
- Happy path: Unregister profile → hotkey removed, ID removed from map
- Edge case: Register profile with hotkey already used → conflict error, no registration
- Edge case: Unregister non-existent profile → no error (or explicit error if preferred)
- Integration: Startup with two profiles → both hotkeys trigger correctly

**Verification:**
- `cargo test --lib shortcut::manager::multi_profile`
- Manual: Create two profiles → press both hotkeys → verify dispatch

---

### Unit 4: Service layer: profile dispatch and resolve

**Goal:** Add dispatch logic that matches on `ShortcutAction` and resolve overrides.

**Requirements:** R3 (polish override works), R8 (fallback safety), R10 (extensibility)

**Dependencies:** Unit 1 (types), shortcut service refactor Unit 1-2 (service layer exists)

**Files:**
- Modify: `apps/desktop/src-tauri/src/services/shortcut.rs`
  - Add `resolve_record_action(profile, settings) -> ResolvedRecordAction`
  - Add `ResolvedRecordAction { polish_provider, polish_model }` struct
  - Add `dispatch_shortcut_action(app, profile_id, state)` function

**Approach:**
- `resolve_record_action(profile, settings)`:
  - If `action = Record { polish_provider: Some(p), polish_model: Some(m) }`:
    - Lookup `p` in `cloud_polish_configs`
    - If found: return resolved provider config + model override
    - If not found: log warning, return None (fallback to global)
  - If `action = Record {}` (no overrides): return None (use global)
- `dispatch_shortcut_action(app, profile_id, state)`:
  - Lookup profile by `profile_id` from settings
  - `match profile.action { Record { .. } => handle_record_trigger(app, state, resolved_action) }`
  - Pass resolved action to `start_recording_sync` or store in call context
- Keep existing `primary_shortcut_action()` for recording mode decision (unchanged)

**Patterns to follow:**
- Pure function style from existing `services/shortcut.rs`
- Context struct pattern for passing resolved data

**Test scenarios:**
- Happy path: Profile with polish override → resolve returns provider config
- Happy path: Profile without override → resolve returns None
- Edge case: Override references deleted config → warning logged, None returned
- Integration: Dispatch with Record action → calls handle_record_trigger

**Verification:**
- `cargo test --lib services::shortcut::resolve`
- `cargo test --lib services::shortcut::dispatch`

---

### Unit 5: IPC commands for profile CRUD

**Goal:** Add Tauri commands for profile management and hotkey capture.

**Requirements:** R2 (create profiles), R5 (conflict detection), R6 (cannot delete default), R9 (headless)

**Dependencies:** Unit 3 (manager registration), Unit 4 (resolve logic)

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/hotkey.rs`
  - `get_shortcut_profiles() -> Vec<ShortcutProfile>`
  - `update_shortcut_profile(profile: ShortcutProfile) -> Result<(), String>`
  - `delete_shortcut_profile(id: String) -> Result<(), String>`
  - `start_hotkey_capture(profile_id: String) -> Result<(), String>`
  - `stop_hotkey_capture(profile_id: String) -> Result<Option<String>, String>`
- Modify: `apps/desktop/src-tauri/src/lib.rs` (register commands)

**Approach:**
- `get_shortcut_profiles`: read from settings, return clone
- `update_shortcut_profile`:
  - If new profile: validate hotkey not in use, register, add to settings
  - If existing profile: unregister old hotkey, register new hotkey, update settings
  - Save settings, emit `SETTINGS_CHANGED`
- `delete_shortcut_profile`:
  - If `id = "default"`: return error `"default_profile_cannot_be_deleted"`
  - Unregister hotkey, remove from settings, save, emit event
- `start_hotkey_capture(profile_id)`: set capture mode for specific profile, store `pending_capture_profile_id`
- `stop_hotkey_capture(profile_id)`: get captured hotkey, bind to profile (update + register), return hotkey string

**Patterns to follow:**
- Existing `start_hotkey_recording` / `stop_hotkey_recording` structure
- `update_settings` pattern: validate → update → save → emit

**Test scenarios:**
- Happy path: Create new profile → added to settings, hotkey registered
- Happy path: Update profile hotkey → old unregistered, new registered
- Happy path: Delete non-default profile → removed, hotkey unregistered
- Error path: Delete default profile → error returned
- Error path: Create profile with conflicting hotkey → error returned
- Integration: Capture hotkey for profile → profile updated with new hotkey

**Verification:**
- `cargo test --lib commands::hotkey::profile_crud`
- Manual: IPC calls via frontend or test script

---

### Unit 6: PreparedRecordingStart polish resolution

**Goal:** Resolve polish overrides into PreparedRecordingStart, pipeline uses resolved values.

**Requirements:** R3 (polish override works), R4 (no override = global), R8 (fallback)

**Dependencies:** Unit 1 (types), Unit 4 (resolve logic)

**Files:**
- Modify: `apps/desktop/src-tauri/src/services/recording_lifecycle.rs`
  - Add `resolved_polish_provider: Option<String>` to `PreparedRecordingStart`
  - Add `resolved_polish_model: Option<String>` to `PreparedRecordingStart`
  - Update `prepare_recording_start()` to accept `ShortcutProfile` and resolve
- Modify: `apps/desktop/src-tauri/src/commands/audio/start.rs`
  - Pass profile to `prepare_recording_start()`
- Modify: `apps/desktop/src-tauri/src/commands/audio/capture.rs`
  - Use `resolved_polish_provider` from `PreparedRecordingStart` instead of settings
- Modify: `apps/desktop/src-tauri/src/commands/audio/polish.rs`
  - Use resolved values from prepared struct

**Approach:**
- `prepare_recording_start()` signature: add `profile: Option<ShortcutProfile>` parameter
- If profile provided and `action = Record { polish_provider, polish_model }`:
  - Call `resolve_record_action(profile, settings)` from services
  - Store resolved values in `PreparedRecordingStart`
- If no profile or no override: store None (pipeline uses global)
- `start_recording_sync`: get profile from triggered profile_id, pass to prepare
- `start_unified_recording`: use prepared values, do NOT re-read settings
- `maybe_polish_transcription_text`: check prepared values first, fallback to global

**Patterns to follow:**
- Single-lock pattern from `prepare_recording_start()`
- Async task captures prepared values pattern

**Test scenarios:**
- Happy path: Profile with polish override → prepared struct contains resolved provider
- Happy path: Profile without override → prepared struct contains None
- Edge case: Override references deleted config → warning, prepared contains None
- Integration: Record with override → polish uses override provider

**Verification:**
- `cargo test --lib services::recording_lifecycle`
- `cargo test --lib commands::audio::polish_resolve`
- Manual: Record with polish override profile → verify polish provider used

---

### Unit 7: Frontend IPC and SettingsContext update

**Goal:** Frontend consumes new IPC commands and `shortcut_profiles` setting.

**Requirements:** R9 (headless compatible, frontend is reactive), R1 (migration transparent)

**Dependencies:** Unit 5 (IPC commands)

**Files:**
- Modify: `apps/desktop/src/lib/tauri.ts`
  - Add `shortcutCommands`: `getProfiles`, `updateProfile`, `deleteProfile`, `startCapture`, `stopCapture`, `cancelCapture`
  - Update `shortcutTriggered` event type to `{ state, profile_id }`
- Modify: `apps/desktop/src/contexts/SettingsContext.tsx`
  - Remove `hotkey` field consumption
  - Add `shortcut_profiles` field consumption
- Modify: `apps/desktop/src/components/ui/hotkey-input.tsx`
  - Pass `profile_id` to capture commands

**Approach:**
- Type definitions match backend command signatures
- SettingsContext: `settings.shortcut_profiles` instead of `settings.hotkey`
- HotkeyInput: new prop `profileId`, calls `startCapture(profileId)`
- Event listener: `shortcutTriggered` now has `{ state, profile_id }`, use `profile_id` to identify which profile triggered

**Patterns to follow:**
- Existing `tauri.ts` command wrapper structure
- Event listener pattern with typed payload

**Test scenarios:**
- Happy path: Call `getProfiles()` → returns profile array
- Happy path: `shortcutTriggered` event → payload has `profile_id`
- Integration: SettingsContext renders profiles list

**Verification:**
- `pnpm --filter @ariatype/desktop build`
- `pnpm --filter @ariatype/shared typecheck`
- Manual: Frontend loads → profiles visible

---

## System-Wide Impact

### Interaction graph

| Entry point | New behavior |
|-------------|--------------|
| `lib.rs` setup | Register all profile hotkeys on startup |
| `handle_hotkey_event` | Dispatch to profile-specific handler |
| `update_settings("hotkey")` | Removed; use profile commands instead |
| `prepare_recording_start` | Accepts profile, resolves polish |
| Frontend hotkey capture | Profile-aware, returns new hotkey for specific profile |

### Error propagation

| Error source | Propagation |
|--------------|-------------|
| Hotkey conflict | `update_shortcut_profile` returns `Err("hotkey_conflict")`, frontend shows toast |
| Delete default | `delete_shortcut_profile` returns `Err("default_profile_cannot_be_deleted")` |
| Override fallback | Warning log, recording proceeds with global settings |

### State lifecycle risks

- Profile hotkey change: must unregister old before registering new, or old hotkey persists (handled in `update_shortcut_profile`)
- Override config deleted: fallback safe, but user may not notice override is ignored (warning log + optional frontend indicator)

### Unchanged invariants

- Recording state machine: `Idle → Recording → ... → Idle` (unchanged)
- Single session: `is_recording` atomic bool (unchanged)
- Cancel hotkey: ESC global (unchanged)
- Recording mode: global hold/toggle (unchanged)

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Shortcut service refactor incomplete | Block Units 4-6 until refactor Units 1-2 done |
| Hotkey registration race condition | Command queue ensures single-threaded processing |
| Migration corrupts settings | Migration function tested with edge cases; saves immediately |
| Override references deleted config | Fallback to global with warning log; no crash |
| Frontend expects old hotkey field | SettingsContext reads `shortcut_profiles`; migration transparent to frontend |

## Verification Evidence

### Backend

```bash
cargo test --lib shortcut::profile_types
cargo test --lib shortcut::manager::multi_profile
cargo test --lib services::shortcut::resolve
cargo test --lib services::shortcut::dispatch
cargo test --lib commands::hotkey::profile_crud
cargo test --lib services::recording_lifecycle
cargo test --lib commands::audio
cargo clippy --all-features -- -D warnings
cargo fmt -- --check
```

### Frontend

```bash
pnpm --filter @ariatype/desktop build
pnpm --filter @ariatype/shared typecheck
```

### Manual

1. Start with old settings → verify migration, default profile works
2. Create profile with polish override → record → verify polish provider used
3. Delete cloud config referenced by override → record → verify fallback + warning
4. Delete default profile → verify rejection
5. Create profile with duplicate hotkey → verify rejection
6. Two profiles active → press one hotkey → verify correct profile dispatch

## Sources & References

- **Origin document:** [context/feat/multi-shortcut/1.0.0/prd/erd.md](../../feat/multi-shortcut/1.0.0/prd/erd.md)
- **Dependency:** [context/plans/active/2026-04-16-002-refactor-shortcut-service-boundaries-plan.md](./2026-04-16-002-refactor-shortcut-service-boundaries-plan.md)
- **Hotkey spec:** [context/spec/hotkey.md](../../spec/hotkey.md)
- **Architecture layers:** [context/architecture/layers.md](../../architecture/layers.md)