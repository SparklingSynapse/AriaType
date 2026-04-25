# Polish Template Shortcut Profiles Feature Specification

## Feature Name

Polish Template Shortcut Profiles

## Version

v1.0.0

## Problem

Users can only configure one global hotkey for recording. The default recording workflow ("Dictate") does not apply polish, while users often need a separate workflow ("Chat") that applies polish with a specific template.

Currently:
- Single `hotkey` → single recording action
- Global `polish_enabled` toggle applies to all recordings
- No way to bind different shortcuts to different polish behaviors

Users want:
- Dictate: Quick voice-to-text, no polish, minimal latency
- Chat: Voice-to-text with polish (e.g., "Formal Style" template for emails, "Agent Prompt" template for AI commands)

## Goal

Allow users to create multiple shortcut profiles, each with:
1. A unique hotkey
2. An optional polish template (None = no polish, Some(template_id) = polish with that template)

The default profile ("Dictate") preserves existing behavior: no polish, just raw STT output.
New profiles ("Chat") specify a polish template; polish uses global provider/model settings.

## First-Principles Model

### Core invariant

**One recording session at any time.** Multiple profiles share the same recording pipeline; the only difference is whether and how polish is applied after transcription.

### Why template ID, not provider/model

Users already manage:
- **Polish templates** in a dedicated UI (PolishTemplatesPage)
- **Polish provider/model** in model settings (PolishSection, CloudPolishSection)

Mixing these concerns in profile creation would:
1. Duplicate existing configuration surfaces
2. Create confusion about where provider settings live
3. Add unnecessary complexity for a feature that only needs "which template to use"

A profile should only answer: **"Do I want polish? Which template?"**
Provider/model is a separate concern managed globally.

### Why remove global polish_enabled

Previously, `polish_enabled` controlled whether polish ran for ALL recordings. Now each profile decides independently:
- `polish_template_id = None` → no polish
- `polish_template_id = Some(...)` → polish enabled

The global toggle becomes redundant and confusing (what happens if global is OFF but a profile has template_id?). Removing it simplifies the mental model.

### Resolution flow

```
Profile.trigger() 
  → PreparedRecordingStart.resolved_polish_template_id
  → Transcription completes
  → maybe_polish_transcription_text()
     → if template_id: 
        - get template.system_prompt
        - use global provider/model (cloud_polish_enabled, cloud_polish_configs)
        - run polish
     → else:
        - skip polish, return raw STT text
```

## Information Architecture

```
┌─────────────────────┐
│    AppSettings       │
│─────────────────────│
│ shortcut_profiles    │  NEW: Map structure with fixed keys
│   .dictate           │  System profile, always exists
│   .chat              │  System profile, always exists
│   .custom?           │  Optional user profile (max 1)
│ polish_model: String │  UNCHANGED: local model fallback
│ active_cloud_polish_ │  UNCHANGED: preferred cloud provider
│   provider: String   │
│ cloud_polish_configs │  UNCHANGED: provider credentials
│ polish_custom_       │  UNCHANGED: custom templates
│   templates[]        │
│ 
│ (removed)            │
│ polish_enabled: bool │  REMOVED: per-profile via template_id
│ polish_selected_     │  REMOVED: each profile has its own template
│   template: String   │
│ cloud_polish_enabled │  REMOVED: implicit from provider config
│ hotkey: String       │  REMOVED: migrated to shortcut_profiles.dictate
└────────┬────────────┘
         │ 3 profiles max
         ▼
┌─────────────────────┐
│  ShortcutProfiles    │  (Map/Object, not Array)
│─────────────────────│
│ dictate: {           │  System profile, undeletable
│   hotkey: String     │
│   action: Record     │
│     template_id: null│  Fixed: no polish
│ }                    │
│                      │
│ chat: {              │  System profile, undeletable
│   hotkey: String     │
│   action: Record     │
│     template_id: Str │  Default: first template, can change, cannot be null
│ }                    │
│                      │
│ custom?: {           │  Optional, user can create/delete (max 1)
│   hotkey: String     │
│   action: Record     │
│     template_id: Str?│  Can be null (no polish) or any template
│ }                    │
└────────┬────────────┘
         │ references (not embeds)
         ▼
┌─────────────────────┐
│  PolishTemplate      │
│  (existing in        │
│   templates.rs or    │
│   polish_custom_     │
│   templates)         │
│─────────────────────│
│ id: String           │  "filler", "formal", or custom UUID
│ system_prompt: String│
└─────────────────────┘
```

**Profile constraints:**

| Profile | Key | Exists | Deletable | template_id | Label |
|---------|-----|--------|-----------|-------------|-------|
| dictate | `dictate` | Always | No | null (fixed) | Dictate |
| chat | `chat` | Always | No | non-null, default first template | Chat |
| custom | `custom` | Optional | Yes | null or any template | Custom |

## Data Contract

### ShortcutProfilesMap (new structure)

```rust
/// Map of shortcut profiles with fixed keys.
/// Stored in settings as an object/map, not an array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutProfilesMap {
    /// System profile: always exists, cannot be deleted.
    /// Fixed polish_template_id = None (no polish).
    pub dictate: ShortcutProfile,
    
    /// System profile: always exists, cannot be deleted.
    /// polish_template_id defaults to first template, can be changed, cannot be None.
    pub chat: ShortcutProfile,
    
    /// Optional user profile: can be created and deleted (max 1).
    /// polish_template_id can be None or any template.
    pub custom: Option<ShortcutProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutProfile {
    /// Hotkey string in handy-keys format (e.g., "Shift+Space", "Cmd+Shift+Space").
    pub hotkey: String,
    
    /// The action this profile triggers.
    pub action: ShortcutAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ShortcutAction {
    Record {
        /// Polish template ID to use for this profile.
        /// - None: Skip polish (dictate behavior)
        /// - Some(template_id): Apply polish with template's prompt + global provider/model
        polish_template_id: Option<String>,
    },
}
```

**Default profiles:**

```rust
impl Default for ShortcutProfilesMap {
    fn default() -> Self {
        Self {
            dictate: ShortcutProfile {
                hotkey: "Shift+Space".to_string(),
                action: ShortcutAction::Record {
                    polish_template_id: None,
                },
            },
            chat: ShortcutProfile {
                hotkey: "".to_string(),  // User must configure
                action: ShortcutAction::Record {
                    polish_template_id: Some("filler".to_string()),  // Default first template
                },
            },
            custom: None,
        }
    }
}
```

**JSON representation:**

```json
{
  "shortcut_profiles": {
    "dictate": {
      "hotkey": "Shift+Space",
      "action": { "Record": { "polish_template_id": null } }
    },
    "chat": {
      "hotkey": "Cmd+Shift+Space",
      "action": { "Record": { "polish_template_id": "filler" } }
    },
    "custom": {
      "hotkey": "Cmd+Alt+Space",
      "action": { "Record": { "polish_template_id": "formal" } }
    }
  }
}
```

### Profile behavior constraints

| Profile | polish_template_id | Constraint | Polish behavior |
|---------|-------------------|------------|-----------------|
| dictate | None | Fixed, cannot change | No polish |
| chat | Some(id) | Cannot be None, defaults to first template | Always polish |
| custom | Option<String> | Can be None or any template | User choice |
```

**Change from existing:** Replace `polish_provider: Option<String>` and `polish_model: Option<String>` with single `polish_template_id: Option<String>`.

### ShortcutProfile (unchanged structure, new default label)

```rust
pub struct ShortcutProfile {
    pub id: String,
    pub label: String,
    pub hotkey: String,
    pub action: ShortcutAction,
}
```

**Default profile after migration:**
```json
{
  "id": "default",
  "label": "Dictate",
  "hotkey": "Shift+Space",
  "action": { "Record": { "polish_template_id": null } }
}
```

### PreparedRecordingStart (modified)

```rust
pub struct PreparedRecordingStart {
    pub task_id: u64,
    pub cloud_stt_enabled: bool,
    pub cloud_stt_config: CloudSttConfig,
    pub language: String,
    // CHANGED: resolved from profile's polish_template_id
    pub resolved_polish_template_id: Option<String>,
}
```

**Removed fields:** `resolved_polish_provider`, `resolved_polish_model` (provider/model now read from global settings at polish execution time).

### AppSettings (modified)

```rust
pub struct AppSettings {
    // ... existing fields unchanged except:
    
    // REMOVED:
    // polish_enabled: bool,
    // polish_selected_template: String,
    // cloud_polish_enabled: bool,
    
    // UNCHANGED (provider/model for polish execution):
    polish_model: String,
    active_cloud_polish_provider: String,
    cloud_polish_configs: HashMap<String, CloudProviderConfig>,
    
    // UNCHANGED (available templates):
    polish_custom_templates: Vec<CustomPolishTemplate>,
    
    // UNCHANGED (from multi-shortcut feature):
    shortcut_profiles: Vec<ShortcutProfile>,
}
```

**Polish provider resolution order:**
1. Check `active_cloud_polish_provider` → lookup in `cloud_polish_configs`
2. If cloud config valid (api_key + model non-empty) → use cloud
3. Else if `polish_model` non-empty → use local
4. Else → skip polish (no provider configured)
```

## Behavior Specification

### Profile behaviors

| Profile | polish_template_id | STT | Polish | Output |
|---------|-------------------|-----|--------|--------|
| Dictate | None | ✓ | skip | Raw transcription |
| Chat #1 | Some("filler") | ✓ | ✓ filler prompt + global provider | Polished text |
| Chat #2 | Some("formal") | ✓ | ✓ formal prompt + global provider | Polished text |

### Polish execution logic

```rust
fn maybe_polish_transcription_text(
    prepared: &PreparedRecordingStart,
    raw_text: String,
    settings: &AppSettings,
) -> Result<String, Error> {
    match prepared.resolved_polish_template_id {
        None => Ok(raw_text),  // No polish (Dictate)
        Some(template_id) => {
            // Get template's system_prompt
            let system_prompt = get_template_prompt(template_id, &settings)?;
            
            // Provider resolution: cloud first, then local fallback
            let provider_type = settings.active_cloud_polish_provider.clone();
            if let Some(config) = settings.cloud_polish_configs.get(&provider_type) {
                if !config.api_key.is_empty() && !config.model.is_empty() {
                    // Use cloud polish
                    return run_cloud_polish(system_prompt, raw_text, config);
                }
            }
            
            // Fallback to local polish if configured
            if !settings.polish_model.is_empty() {
                run_local_polish(system_prompt, raw_text, &settings.polish_model)
            } else {
                // No polish provider configured, skip
                warn!("polish_skipped-no_provider_configured");
                Ok(raw_text)
            }
        }
    }
}

fn get_template_prompt(template_id: &str, settings: &AppSettings) -> Result<String, Error> {
    // Check built-in templates first
    if let Some(built_in) = get_template_by_id(template_id) {
        return Ok(built_in.system_prompt);
    }
    
    // Check custom templates
    if let Some(custom) = settings.polish_custom_templates.iter().find(|t| t.id == template_id) {
        return Ok(custom.system_prompt.clone());
    }
    
    // Template missing: use first available template or default
    warn!("template_not_found_fallback", template_id);
    Ok(get_template_by_id("filler")
        .map(|t| t.system_prompt)
        .unwrap_or(DEFAULT_POLISH_PROMPT))
}
```

### Global polish settings role

| Setting | Role in new design |
|---------|-------------------|
| `active_cloud_polish_provider` | Which cloud provider to use (if configured) |
| `cloud_polish_configs` | Provider credentials, checked at polish execution |
| `polish_model` | Local model for fallback (when cloud not configured) |
| `polish_custom_templates` | Available custom templates for profile selection |

**REMOVED fields:**
| Setting | Reason |
|---------|--------|
| `polish_enabled` | Per-profile via `polish_template_id` |
| `polish_selected_template` | Each profile specifies its own template_id |
| `cloud_polish_enabled` | Implicit: cloud used if `active_cloud_polish_provider` has valid config |

### Recording mode (unchanged)

`recording_mode` (hold/toggle) remains global. All profiles share the same mode.

## Settings Migration

### From single hotkey to profiles map

**Old format:**
```json
{
  "hotkey": "Shift+Space",
  "polish_enabled": true,
  "polish_selected_template": "filler",
  "cloud_polish_enabled": true
}
```

**New format:**
```json
{
  "shortcut_profiles": {
    "dictate": {
      "hotkey": "Shift+Space",
      "action": { "Record": { "polish_template_id": null } }
    },
    "chat": {
      "hotkey": "",
      "action": { "Record": { "polish_template_id": "filler" } }
    }
  }
}
```

**Migration logic:**
1. Create `shortcut_profiles` map:
   - `dictate`: hotkey from old `hotkey` field (or "Shift+Space" default), template_id=null
   - `chat`: hotkey="" (user must configure), template_id=first available template ("filler")
   - `custom`: None (not created)
2. Remove obsolete fields:
   - `hotkey`
   - `polish_enabled`
   - `polish_selected_template`
   - `cloud_polish_enabled`
3. If `shortcut_profiles` was an array (old multi-shortcut format):
   - Convert first element to `dictate` profile
   - Convert second element to `chat` profile if exists
   - Convert remaining to `custom` (max 1)
4. Save migrated settings

### Backward compatibility

| Scenario | Migration result |
|----------|-----------------|
| User had single hotkey | `dictate.hotkey` = old hotkey, `chat.hotkey` = "" |
| User had polish_enabled=true | Behavior changes: `dictate` has no polish, user must use `chat` |
| User had multiple profiles (array) | First → dictate, second → chat, third → custom |

## IPC Commands

### Profile management commands

| Command | Signature | Purpose |
|---------|-----------|---------|
| `get_shortcut_profiles` | `() -> ShortcutProfilesMap` | Returns profiles map |
| `update_shortcut_profile` | `(key: String, profile: ShortcutProfile) -> Result<(), String>` | Update specific profile |
| `create_custom_profile` | `(profile: ShortcutProfile) -> Result<(), String>` | Create custom profile (max 1) |
| `delete_custom_profile` | `() -> Result<(), String>` | Delete custom profile |
| `get_polish_templates` | `() -> Vec<PolishTemplate>` | List available templates |

### Profile update constraints

| Profile key | Allowed operations |
|-------------|-------------------|
| `dictate` | Update hotkey only |
| `chat` | Update hotkey + template_id (cannot be null) |
| `custom` | Create, update, delete |

### Error cases

| Error | Condition |
|-------|-----------|
| `cannot_update_dictate_template` | Attempt to change dictate's polish_template_id |
| `cannot_delete_system_profile` | Attempt to delete dictate or chat |
| `custom_profile_already_exists` | Create custom when custom already exists |
| `chat_template_cannot_be_null` | Attempt to set chat's template_id to null |

### Profile payload format

```typescript
interface ShortcutProfilesMap {
  dictate: ShortcutProfile;
  chat: ShortcutProfile;
  custom?: ShortcutProfile;
}

interface ShortcutProfile {
  hotkey: string;
  action: {
    Record?: {
      polish_template_id?: string | null;
    };
  };
}
```

## Frontend UI

### HotkeySettings.tsx structure

```
┌─ Shortcut Profiles ─────────────────────────────────────┐
│                                                          │
│  Dictate (system)                                        │
│  ├─ Hotkey: [Shift+Space]         [Change]              │
│  ├─ Polish: None (fixed)                                 │
│  └──────────────────────────────────────────────────────│
│                                                          │
│  Chat (system)                                           │
│  ├─ Hotkey: [Cmd+Shift+Space]     [Change]              │
│  ├─ Polish Template: [Filler ▼]   (cannot be None)      │
│  └──────────────────────────────────────────────────────│
│                                                          │
│  Custom                                                  │
│  ├─ Hotkey: [Cmd+Alt+Space]       [Change]              │
│  ├─ Polish Template: [None ▼]     (can be None)         │
│  ├─                                 [Delete]             │
│  └──────────────────────────────────────────────────────│
│                                                          │
│  (if no custom exists)                                   │
│  [+ Create Custom Profile]                               │
│                                                          │
│  ──────────────────────────────────────────────────────│
│                                                          │
│  Recording Mode                                          │
│  [Hold]  [Toggle]                                        │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Template dropdown per profile

| Profile | Dropdown options | Selection constraint |
|---------|-----------------|---------------------|
| dictate | None (disabled, shows "None (fixed)") | Cannot change |
| chat | All templates | Cannot select None |
| custom | None + All templates | Can select None or any template |

### Create Custom Modal

```
┌─ Create Custom Profile ─────────────────────────────────┐
│                                                          │
│  Hotkey: [Press keys to capture...]                      │
│                                                          │
│  Polish Template:                                        │
│  ┌─────────────────────────────────────────────────────┐│
│  │ ○ None (no polish)                                  ││
│  │ ● Remove Fillers    - Remove filler words           ││
│  │   Formal Style      - Professional written style    ││
│  │   Make Concise      - Shorten and simplify          ││
│  │   Agent Prompt      - Structured markdown for AI    ││
│  │                                                      ││
│  │   My Custom Template #1                             ││
│  └─────────────────────────────────────────────────────┘│
│                                                          │
│                              [Cancel]  [Create]          │
└──────────────────────────────────────────────────────────┘
```

## Acceptance Criteria

1. **Fixed profile keys**: Profiles use map structure with keys `dictate`, `chat`, `custom` (optional).
2. **Dictate always exists**: `dictate` profile cannot be deleted, always has template_id=null.
3. **Chat always exists**: `chat` profile cannot be deleted, template_id defaults to first template, cannot be null.
4. **Custom optional**: User can create at most one `custom` profile, can delete it.
5. **Hotkey configuration**: Each profile has its own hotkey, conflicts detected across all three.
6. **Template selection**: 
   - dictate: template fixed to None (no UI to change)
   - chat: can select any template (dropdown excludes None)
   - custom: can select None or any template
7. **Provider resolution**: Chat/Custom profiles use global polish provider (cloud first, local fallback).
8. **Migration preserves hotkey**: Old single hotkey → dictate.hotkey.
9. **No orphan fields**: Obsolete fields removed during migration.

## BDD Scenarios

### Scenario: Migration from single hotkey

```gherkin
Given existing settings with hotkey="Shift+Space" and polish_enabled=true
When the app starts with new version
Then shortcut_profiles.dictate.hotkey = "Shift+Space"
And shortcut_profiles.dictate.action.Record.polish_template_id = null
And shortcut_profiles.chat.hotkey = ""
And shortcut_profiles.chat.action.Record.polish_template_id = "filler"
And obsolete fields removed
```

### Scenario: Dictate profile cannot change template

```gherkin
Given shortcut_profiles.dictate exists
When user attempts to update dictate with polish_template_id="filler"
Then update rejected with error "cannot_update_dictate_template"
```

### Scenario: Chat profile cannot have null template

```gherkin
Given shortcut_profiles.chat exists
When user attempts to update chat with polish_template_id=null
Then update rejected with error "chat_template_cannot_be_null"
```

### Scenario: Create custom profile

```gherkin
Given no custom profile exists
When user creates custom with hotkey="Cmd+Alt+Space" and template="formal"
Then shortcut_profiles.custom is created
And hotkey "Cmd+Alt+Space" is registered
```

### Scenario: Cannot create second custom profile

```gherkin
Given shortcut_profiles.custom exists
When user attempts to create another custom profile
Then creation rejected with error "custom_profile_already_exists"
```

### Scenario: Delete custom profile

```gherkin
Given shortcut_profiles.custom exists
When user deletes custom profile
Then shortcut_profiles.custom is removed
And its hotkey is unregistered
```

### Scenario: Cannot delete system profiles

```gherkin
Given shortcut_profiles.dictate and shortcut_profiles.chat exist
When user attempts to delete either
Then deletion rejected with error "cannot_delete_system_profile"
```

### Scenario: Chat uses polish

```gherkin
Given shortcut_profiles.chat with template_id="filler"
And valid cloud polish provider configured
When user presses chat hotkey and speaks "hey check this out"
Then STT produces "hey check this out"
And polish applies filler template prompt
And polished text "check this out" is output
```

### Scenario: Dictate skips polish

```gherkin
Given shortcut_profiles.dictate with template_id=null
When user presses dictate hotkey and speaks "hello world"
Then STT produces "hello world"
And polish is skipped
And raw text "hello world" is output
```

### Scenario: Create Chat profile with template

```gherkin
Given default profile "Dictate" with hotkey="Shift+Space"
When user creates profile with label="Chat", hotkey="Cmd+Shift+Space", template="formal"
Then profile is saved and Cmd+Shift+Space is registered
When user presses Cmd+Shift+Space and speaks "hey, check this out"
Then STT produces "hey, check this out"
And polish applies formal template prompt: "Could you please review this?"
And polished text is injected
```

### Scenario: Template references deleted custom template

```gherkin
Given profile with polish_template_id="custom-abc-123"
And custom template "custom-abc-123" is deleted from polish_custom_templates
When user triggers recording with this profile
Then recording proceeds
And polish uses fallback template (first built-in template)
And warning logged: "template_not_found_fallback: custom-abc-123"
```

### Scenario: Two shortcuts cannot share hotkey

```gherkin
Given default profile with hotkey="Shift+Space"
When user creates new profile with hotkey="Shift+Space"
Then creation rejected with error "Hotkey 'Shift+Space' already used by profile 'default'"
```

### Scenario: Delete default profile rejected

```gherkin
Given default profile exists
When user attempts to delete profile with id="default"
Then deletion rejected with error "default_profile_cannot_be_deleted"
```

### Scenario: Chat profile uses cloud polish when configured

```gherkin
Given active_cloud_polish_provider="anthropic" and cloud_polish_configs has valid anthropic entry
And Chat profile with template="filler"
When user triggers recording with Chat profile
Then polish runs using anthropic cloud provider
And filler template's system_prompt is used
```

### Scenario: Chat profile falls back to local polish when cloud not configured

```gherkin
Given active_cloud_polish_provider="anthropic" but cloud_polish_configs has no anthropic entry
And polish_model="gemma-2-9b" is configured
And Chat profile with template="filler"
When user triggers recording with Chat profile
Then polish runs using local gemma-2-9b model
And filler template's system_prompt is used
```

### Scenario: Chat profile skips polish when no provider configured

```gherkin
Given active_cloud_polish_provider="" and polish_model=""
And Chat profile with template="filler"
When user triggers recording with Chat profile
Then polish is skipped
And warning logged: "polish_skipped-no_provider_configured"
And raw STT text is output
```

## File Change Map

### Backend (modified)

| File | Change |
|------|--------|
| `shortcut/profile_types.rs` | Replace array-based profiles with `ShortcutProfilesMap` (dictate/chat/custom) |
| `shortcut/manager.rs` | Register hotkeys from map, handle profile key in trigger events |
| `services/shortcut.rs` | Update resolve logic for map structure |
| `services/recording_lifecycle.rs` | Use `resolved_polish_template_id` from profile |
| `commands/audio/polish.rs` | Template → prompt, cloud/local resolution |
| `commands/settings/mod.rs` | Remove obsolete fields, add map migration |
| `commands/hotkey.rs` | New commands: `get_profiles`, `update_profile`, `create_custom`, `delete_custom` |

### Frontend (modified)

| File | Change |
|------|--------|
| `lib/tauri.ts` | `ShortcutProfilesMap` type, new command signatures |
| `components/Home/HotkeySettings.tsx` | Render dictate/chat/custom sections, template dropdowns |
| `contexts/SettingsContext.tsx` | Consume `shortcut_profiles` map |

## Glossary

| Term | Definition |
|------|------------|
| Profile map | Fixed-key structure: `{ dictate, chat, custom? }` |
| System profile | `dictate` or `chat`, cannot be deleted |
| Custom profile | Optional user-created profile, max 1 |
| Template | Built-in or custom polish template defining system_prompt |
| Provider resolution | Cloud first → local fallback → skip with warning |

## Verification

### Backend

```bash
cargo test --lib shortcut::profile_types
cargo test --lib services::shortcut::resolve
cargo test --lib services::recording_lifecycle
cargo test --lib commands::audio::polish
cargo clippy --all-features -- -D warnings
cargo fmt -- --check
```

### Frontend

```bash
pnpm --filter @ariatype/desktop build
pnpm --filter @ariatype/shared typecheck
```

### Manual

1. Upgrade from old version → verify default profile "Dictate", no polish
2. Create Chat profile with template → verify polish uses template prompt
3. Delete custom template referenced by profile → verify fallback + warning
4. Create profile with duplicate hotkey → verify rejection
5. Delete default profile → verify rejection
6. Trigger Dictate → verify raw output
7. Trigger Chat → verify polished output with correct template

## Glossary

| Term | Definition |
|------|------------|
| Profile | Named binding of hotkey to recording action with optional polish template |
| Dictate | Default profile, no polish, raw STT output |
| Chat | User-created profile, applies polish with specified template |
| Template | Built-in or custom polish template defining system_prompt |
| Global polish settings | Provider/model configuration shared by all Chat profiles |