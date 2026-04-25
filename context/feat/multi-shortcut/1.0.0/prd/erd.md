# Multi-Shortcut Feature Specification

## Feature Name

Multi-Shortcut Profiles

## Version

v1.0.0

## Problem

Users can only configure one global hotkey. That single hotkey always triggers the same recording pipeline with globally configured STT and Polish settings. There is no way to bind different shortcuts to different post-processing behaviors — for example, one shortcut for raw transcription and another for transcription with a specific polish template.

The current architecture hardcodes a 1:1 mapping: **one hotkey → one action (record)**. All downstream parameters (STT provider, polish provider) come from global settings, making per-shortcut customization impossible without changing settings each time.

## Goal

Support multiple keyboard shortcuts ("profiles"), each bound to a distinct action. The default profiles provide two common use cases out of the box.

**Extensibility goal:** adding a new action kind (e.g., MeetingRecord) should be additive — a new enum variant plus a new handler — without modifying existing action code paths.

## First-Principles Model

### Core questions this feature answers

1. **What does a shortcut do?** — It triggers an *action*. Each action kind carries exactly the data it needs, nothing more.
2. **Can two shortcuts record at the same time?** — No. The single-session invariant is preserved: only one recording session exists at any time, regardless of which profile triggered it.

### Why fixed-key map structure, not dynamic Vec

**Decision:** Use `ShortcutProfilesMap { dictate, chat, custom? }` with fixed keys, not `Vec<ShortcutProfile>` with dynamic IDs.

**Reasons:**
1. **Simpler validation** — No need for "default profile cannot be deleted" logic. System profiles (dictate/chat) are always present.
2. **Clearer UX** — Users understand "Dictate mode" vs "Chat mode" vs "Custom". No arbitrary profile naming/labeling.
3. **Fixed constraints** — Dictate's `polish_template_id` is always None (no polish). Chat's `polish_template_id` is always Some (must have template). These constraints are enforced by the fixed structure.
4. **Max 1 custom profile** — Optional `custom` field enforces "one extra profile" limit without complex count validation.

**Trade-off:** Adding more profile types requires modifying the map structure. This is acceptable because profile types are semantic (different behaviors), not arbitrary user creations.

### Why polish_template_id, not polish_provider + polish_model

**Decision:** Each profile stores `polish_template_id: Option<String>`, referencing the existing polish template system. Provider and model come from global settings.

**Reasons:**
1. **Reuse existing abstraction** — Polish templates already encapsulate the prompt + behavior. No need to duplicate provider/model selection per profile.
2. **Single source of truth** — Cloud polish provider/model is a global concern. Profiles only choose *which template* to apply, not which cloud service.
3. **Simpler UI** — User picks a template (filler, formal, custom), not a provider + model combination.

**Override resolution:** When profile triggers recording:
- If `polish_template_id = Some(id)`: Use that template's `system_prompt` + global polish provider/model
- If `polish_template_id = None`: Skip polish entirely (dictate behavior)

### Why resolve into PreparedRecordingStart

When a profile triggers recording, the template_id needs to reach `maybe_polish_transcription_text` deep in the pipeline. Resolve at preparation time in `prepare_recording_start()` and store `resolved_polish_template_id` in `PreparedRecordingStart`.

The async recording task reads the resolved template_id directly. Pipeline doesn't need to know whether the value came from a profile or global settings.

### Stable vs. changing surfaces

| Surface | Stability | Reason |
|---------|-----------|--------|
| ShortcutProfilesMap (dictate/chat/custom) | Stable | Core abstraction, fixed keys |
| ShortcutAction enum | Extensible | New variants for new action kinds |
| ShortcutProfile (hotkey + action) | Stable | Simple tuple |
| PreparedRecordingStart extension | Stable | Existing pattern, adds resolved field |
| Settings format | Stable after migration | One migration, then stable |
| Frontend profile UI | Evolving | UX may iterate, IPC contract stable |

## Information Architecture

```
┌─────────────────────┐
│    AppSettings       │
│─────────────────────│
│ recording_mode: Str  │  (global, unchanged)
│ shortcut_profiles:   │  (NEW: map structure)
│   dictate: Profile   │  Cmd+Slash, no polish
│   chat: Profile      │  Opt+Slash, default template
│   custom?: Profile   │  (optional, max 1)
│ ...                  │
└─────────────────────┘

┌─────────────────────┐
│  ShortcutProfile     │
│─────────────────────│
│ hotkey: String       │  "Cmd+Slash", "Opt+Slash"
│ action: ShortcutAction│
└────────┬────────────┘
         │
         ▼
┌─────────────────────┐
│  ShortcutAction      │  (enum)
│─────────────────────│
│ Record {             │
│   polish_template_id │  Option<String> — template ID
│ }                    │  None = skip polish
│                      │  Some(id) = use template prompt
│ (future variants)    │  + global provider/model
│ MeetingRecord { … }  │
└─────────────────────┘
         │ polish_template_id references
         ▼
┌─────────────────────┐
│ PolishTemplate       │  (existing system)
│─────────────────────│
│ id: String           │  "filler", "formal", "user_xxx"
│ system_prompt: String│
└─────────────────────┘
```

## Data Contract

### ShortcutProfilesMap

```rust
struct ShortcutProfilesMap {
    dictate: ShortcutProfile,   // Always exists, polish_template_id = None
    chat: ShortcutProfile,      // Always exists, polish_template_id = Some
    custom: Option<ShortcutProfile>,  // Optional, max 1
}
```

**Constraints:**
- `dictate` and `chat` cannot be deleted
- `dictate.polish_template_id` must be `None` (enforced by backend)
- `chat.polish_template_id` must be `Some` (enforced by backend)
- `custom.polish_template_id` can be `None` or `Some`
- All hotkeys must be unique across profiles
- Hotkey must pass validation rules from `context/spec/hotkey.md`

### ShortcutProfile

```rust
struct ShortcutProfile {
    hotkey: String,
    action: ShortcutAction,
}
```

### ShortcutAction

```rust
enum ShortcutAction {
    Record {
        polish_template_id: Option<String>,
    },
}
```

**Resolution rule:** 
- `polish_template_id = Some(id)`: Use template's `system_prompt` + global `active_cloud_polish_provider` + `cloud_polish_configs`
- `polish_template_id = None`: Skip polish (dictate behavior)

### Default Values

| Profile | Hotkey | Template |
|---------|--------|----------|
| dictate | `Cmd+Slash` | None (no polish) |
| chat | `Opt+Slash` | `filler` |

### PreparedRecordingStart Extension

```rust
struct PreparedRecordingStart {
    task_id: u64,
    cloud_stt_enabled: bool,
    cloud_stt_config: CloudSttConfig,
    language: String,
    // NEW: resolved at preparation time
    resolved_polish_template_id: Option<String>,
}
```

### Settings Format

```rust
struct AppSettings {
    shortcut_profiles: ShortcutProfilesMap,  // NEW: replaces hotkey
    recording_mode: String,                   // Unchanged
    active_cloud_polish_provider: String,    // Unchanged
    cloud_polish_configs: HashMap<...>,      // Unchanged
    polish_custom_templates: Vec<...>,       // Unchanged
    // ... all other fields unchanged
}
```

**Removed fields:**
- `hotkey: String` — migrated to `shortcut_profiles.dictate.hotkey`
- `polish_enabled: bool` — polish is now per-profile via template_id
- `cloud_polish_enabled: bool` — redundant with provider config validity
- `polish_selected_template: String` — templates are now per-profile

### Settings Migration

Old format:
```json
{
  "hotkey": "Shift+Space",
  "polish_enabled": true,
  "polish_selected_template": "filler"
}
```

New format:
```json
{
  "shortcut_profiles": {
    "dictate": {
      "hotkey": "Shift+Space",
      "action": { "Record": { "polish_template_id": null } }
    },
    "chat": {
      "hotkey": "Opt+Slash",
      "action": { "Record": { "polish_template_id": "filler" } }
    },
    "custom": null
  }
}
```

Migration logic:
1. `hotkey` → `shortcut_profiles.dictate.hotkey`
2. `polish_selected_template` → `shortcut_profiles.chat.action.Record.polish_template_id`
3. Remove obsolete fields: `hotkey`, `polish_enabled`, `cloud_polish_enabled`, `polish_selected_template`

### IPC Events Changed

| Event | Old Payload | New Payload | Backward Compat |
|-------|-------------|-------------|-----------------|
| `shortcut-triggered` | `"pressed"` / `"released"` | `{ state, profile_id }` | **Breaking** |
| `shortcut-registration-failed` | `string` | `{ error, profile_id }` | Extended |

### IPC Commands

| Command | Signature | Purpose |
|---------|-----------|---------|
| `get_shortcut_profiles` | `() -> ShortcutProfilesMap` | Get all profiles |
| `update_shortcut_profile` | `(key, profile) -> ()` | Update by key (dictate/chat/custom) |
| `create_custom_profile` | `(profile) -> ()` | Create custom (fails if exists) |
| `delete_custom_profile` | `() -> ()` | Delete custom |
| `start_hotkey_capture` | `(profileKey) -> ()` | Begin capture for profile |
| `stop_hotkey_capture` | `(profileKey) -> String` | Complete capture, return hotkey |
| `cancel_hotkey_capture` | `() -> ()` | Cancel capture |

## Acceptance Criteria

1. **Migration preserved**: Existing hotkey → dictate profile
2. **Two default profiles work**: Cmd+Slash → dictate (no polish), Opt+Slash → chat (polish with filler)
3. **Custom profile optional**: Can create/delete one custom profile
4. **Template per-profile**: Each profile picks its polish template
5. **Hotkey uniqueness**: Same hotkey cannot be assigned to multiple profiles
6. **Global polish config**: Provider/model from `cloud_polish_configs`, template prompt from profile
7. **Single session**: Only one recording at any time
8. **Fallback safety**: Invalid template_id → skip polish, log warning
9. **Headless compatible**: All commands accessible via Tauri IPC

## BDD Scenarios

### Scenario: Default profiles work out of box

```gherkin
Given fresh install with default settings
When user presses Cmd+Slash
Then recording starts with no polish
When user presses Opt+Slash
Then recording starts with polish using filler template + global provider
```

### Scenario: Migration from single hotkey

```gherkin
Given existing settings with hotkey="Cmd+Space"
When app starts with new version
Then shortcut_profiles.dictate.hotkey = "Cmd+Space"
And shortcut_profiles.chat.hotkey = "Opt+Slash"
And pressing Cmd+Space triggers dictate (no polish)
```

### Scenario: Create custom profile

```gherkin
Given no custom profile exists
When user creates custom with hotkey="Ctrl+Space", template="formal"
Then custom profile is saved and hotkey registered
When user presses Ctrl+Space
Then recording uses formal template
```

### Scenario: Hotkey conflict rejected

```gherkin
Given dictate.hotkey="Cmd+Slash"
When user tries to set chat.hotkey="Cmd+Slash"
Then error "hotkey_conflict:dictate"
And chat profile unchanged
```

### Scenario: Delete custom profile

```gherkin
Given custom profile exists
When user deletes it
Then custom = null
And hotkey unregistered
And dictate/chat unchanged
```

### Scenario: Template fallback

```gherkin
Given chat profile with template_id="deleted_template"
And polish_custom_templates does not contain "deleted_template"
When user triggers chat profile
Then recording starts, polish skipped
And warning logged: "template_not_found: deleted_template"
```

## Verification

### Backend

```bash
cargo test --lib shortcut::profile_types
cargo test --lib commands::hotkey
cargo clippy --all-features -- -D warnings
```

### Frontend

```bash
pnpm --filter @ariatype/desktop build
pnpm --filter @ariatype/shared typecheck
```

### Manual

1. Fresh install → verify Cmd+Slash and Opt+Slash work
2. Migration from old version → verify hotkey preserved in dictate
3. Create custom profile → verify third hotkey works
4. Delete custom → verify removed
5. Conflict detection → verify rejection
6. Invalid template → verify fallback + warning

## Architecture Decisions

### ADR-1: Fixed-key map structure

**Decision:** `ShortcutProfilesMap { dictate, chat, custom? }` not `Vec<ShortcutProfile>`.

**Consequence:** 
- System profiles always exist, cannot be deleted
- Clear semantic meaning: dictate (no polish) vs chat (polish)
- Max 1 custom enforced by Optional field
- Adding new profile types requires struct change (acceptable trade-off)

### ADR-2: polish_template_id references template system

**Decision:** Profile stores template ID, not provider/model. Provider/model from global settings.

**Consequence:**
- Single source of truth for provider config
- Templates reusable across profiles
- Simpler UI (pick template, not provider+model)
- Template deletion → fallback to no polish

### ADR-3: Resolve into PreparedRecordingStart

**Decision:** Resolve template_id at preparation time, store in PreparedRecordingStart.

**Consequence:**
- Pipeline reads resolved value directly
- No session-state indirection
- One resolve location (prepare_recording_start)

### ADR-4: Global recording mode

**Decision:** `recording_mode` remains global. All profiles share hold/toggle behavior.

**Consequence:** Simpler UX. Per-profile mode can be added later if needed.

## File Change Map

### New files

| File | Purpose |
|------|---------|
| `shortcut/profile_types.rs` | ShortcutProfilesMap, ShortcutProfile, ShortcutAction |

### Modified files (backend)

| File | Change |
|------|--------|
| `shortcut/mod.rs` | Export profile_types |
| `shortcut/manager.rs` | HashMap for registered profiles, profile-aware dispatch |
| `shortcut/types.rs` | ShortcutEvent with profile_id |
| `services/shortcut.rs` | Profile helpers, hotkey uniqueness validation |
| `services/recording_lifecycle.rs` | resolved_polish_template_id in PreparedRecordingStart |
| `commands/hotkey.rs` | Profile CRUD commands, capture commands |
| `commands/settings/mod.rs` | shortcut_profiles field, migration logic, removed obsolete fields |
| `commands/model.rs` | Removed select/get template commands (now per-profile) |
| `events/mod.rs` | Updated event payloads |
| `lib.rs` | Startup: register all profiles with hotkeys |

### Modified files (frontend)

| File | Change |
|------|--------|
| `lib/tauri.ts` | ShortcutProfilesMap type, new commands, updated events |
| `components/Home/HotkeySettings.tsx` | Profile sections UI |
| `components/ui/hotkey-input.tsx` | Profile-aware capture |
| `hooks/useRecording.ts` | Read dictate hotkey for display |

## Glossary

| Term | Definition |
|------|------------|
| Dictate profile | System profile with no polish (raw transcription) |
| Chat profile | System profile with polish template |
| Custom profile | Optional user profile with any template |
| Template | Polish prompt configuration (filler, formal, user-defined) |