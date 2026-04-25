---
title: Polish Template Shortcut Profiles Implementation
type: feat
status: active
date: 2026-04-21
origin: context/brainstorms/2026-04-21-polish-template-shortcut-profiles.md
---

# Polish Template Shortcut Profiles Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Allow users to create multiple shortcut profiles, each binding a hotkey to a polish template. Default "Dictate" profile skips polish; new "Chat" profiles apply polish with specified template.

**Architecture:** Replace array-based profiles with map structure: `ShortcutProfilesMap { dictate, chat, custom? }`. Fixed keys with constraints: `dictate` (template_id=null, fixed), `chat` (template_id non-null, defaults to first template), `custom` (optional, max 1, any template). Polish execution uses template's prompt + provider resolution (cloud first, local fallback). Remove `polish_enabled`, `polish_selected_template`, `cloud_polish_enabled`. Frontend renders three profile sections with appropriate constraints.

**Tech Stack:** Rust (Tauri backend), TypeScript/React (frontend), serde for JSON serialization.

---

## Implementation Units

### Unit 1: Create ShortcutProfilesMap structure

**Goal:** Replace array-based profiles with fixed-key map structure (dictate/chat/custom).

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/profile_types.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/__test__/mod.rs`

---

**Step 1: Write failing test for new map structure**

Add to `apps/desktop/src-tauri/src/shortcut/__test__/mod.rs`:

```rust
#[test]
fn profiles_map_serializes_with_fixed_keys() {
    let profiles = ShortcutProfilesMap {
        dictate: ShortcutProfile {
            hotkey: "Shift+Space".to_string(),
            action: ShortcutAction::Record {
                polish_template_id: None,
            },
        },
        chat: ShortcutProfile {
            hotkey: "Cmd+Space".to_string(),
            action: ShortcutAction::Record {
                polish_template_id: Some("filler".to_string()),
            },
        },
        custom: None,
    };
    
    let json = serde_json::to_string(&profiles).unwrap();
    assert!(json.contains("\"dictate\""));
    assert!(json.contains("\"chat\""));
    assert!(!json.contains("\"custom\"")); // None not serialized
}

#[test]
fn profiles_map_with_custom_serializes() {
    let profiles = ShortcutProfilesMap {
        dictate: ShortcutProfile::default_dictate(),
        chat: ShortcutProfile::default_chat(),
        custom: Some(ShortcutProfile {
            hotkey: "Cmd+Alt+Space".to_string(),
            action: ShortcutAction::Record {
                polish_template_id: Some("formal".to_string()),
            },
        }),
    };
    
    let json = serde_json::to_string(&profiles).unwrap();
    assert!(json.contains("\"custom\""));
}
```

---

**Step 2: Run test to verify failure**

Run: `cargo test --lib shortcut::__test__::profiles_map_serializes_with_fixed_keys`

Expected: FAIL — `ShortcutProfilesMap` type doesn't exist

---

**Step 3: Create ShortcutProfilesMap and updated ShortcutProfile**

Edit `apps/desktop/src-tauri/src/shortcut/profile_types.rs`:

```rust
/// Map of shortcut profiles with fixed keys.
/// Stored in settings as an object/map, not an array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutProfilesMap {
    /// System profile: always exists, cannot be deleted.
    /// Fixed polish_template_id = None (no polish).
    pub dictate: ShortcutProfile,
    
    /// System profile: always exists, cannot be deleted.
    /// polish_template_id defaults to first template, cannot be None.
    pub chat: ShortcutProfile,
    
    /// Optional user profile: can be created and deleted (max 1).
    /// polish_template_id can be None or any template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<ShortcutProfile>,
}

/// Single shortcut profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutProfile {
    /// Hotkey string in handy-keys format.
    pub hotkey: String,
    /// The action this profile triggers.
    pub action: ShortcutAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ShortcutAction {
    Record {
        polish_template_id: Option<String>,
    },
}
```

---

**Step 4: Add default constructors**

```rust
impl ShortcutProfilesMap {
    pub fn default() -> Self {
        Self {
            dictate: ShortcutProfile::default_dictate(),
            chat: ShortcutProfile::default_chat(),
            custom: None,
        }
    }
    
    pub fn with_migration_hotkey(hotkey: String) -> Self {
        Self {
            dictate: ShortcutProfile {
                hotkey,
                action: ShortcutAction::Record { polish_template_id: None },
            },
            chat: ShortcutProfile::default_chat(),
            custom: None,
        }
    }
}

impl ShortcutProfile {
    pub fn default_dictate() -> Self {
        Self {
            hotkey: "Shift+Space".to_string(),
            action: ShortcutAction::Record { polish_template_id: None },
        }
    }
    
    pub fn default_chat() -> Self {
        Self {
            hotkey: "".to_string(), // User must configure
            action: ShortcutAction::Record {
                polish_template_id: Some("filler".to_string()),
            },
        }
    }
}

impl Default for ShortcutAction {
    fn default() -> Self {
        ShortcutAction::Record { polish_template_id: None }
    }
}
```

---

**Step 5: Update existing tests**

Update serialization tests to use new structure:

```rust
#[test]
fn profile_serialization_roundtrip() {
    let profile = ShortcutProfile {
        hotkey: "Cmd+Shift+Space".to_string(),
        action: ShortcutAction::Record {
            polish_template_id: Some("filler".to_string()),
        },
    };

    let json = serde_json::to_string(&profile).unwrap();
    let decoded: ShortcutProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(profile, decoded);
}

#[test]
fn action_serializes_to_pascal_case() {
    let action = ShortcutAction::Record { polish_template_id: None };
    let json = serde_json::to_string(&action).unwrap();
    assert_eq!(json, r#"{"Record":{"polish_template_id":null}}"#);
}

#[test]
fn profiles_map_deserializes_missing_custom() {
    let json = r#"{"dictate":{"hotkey":"Shift+Space","action":{"Record":{"polish_template_id":null}}},"chat":{"hotkey":"","action":{"Record":{"polish_template_id":"filler"}}}}"#;
    let profiles: ShortcutProfilesMap = serde_json::from_str(json).unwrap();
    assert!(profiles.custom.is_none());
}
```

---

**Step 6: Run tests to verify**

Run: `cargo test --lib shortcut::profile_types`

Expected: All tests PASS

---

**Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/shortcut/profile_types.rs
git commit -m "refactor(shortcut): replace array with ShortcutProfilesMap {dictate, chat, custom}"
```

---

### Unit 2: Update shortcut service for map structure

**Goal:** Update resolve and validation functions for map-based profiles.

**Files:**
- Modify: `apps/desktop/src-tauri/src/services/shortcut.rs`
- Modify: `apps/desktop/src-tauri/src/services/shortcut.rs` tests

---

**Step 1: Write failing test for map-based resolution**

Add to tests:

```rust
#[test]
fn resolve_template_from_profile_key() {
    let profiles = ShortcutProfilesMap::default();
    
    let template = resolve_profile_template(&profiles, "dictate");
    assert!(template.is_none());
    
    let template = resolve_profile_template(&profiles, "chat");
    assert_eq!(template, Some("filler".to_string()));
}
```

---

**Step 2: Run test to verify failure**

Run: `cargo test --lib services::shortcut::resolve_template_from_profile_key`

Expected: FAIL — function signature mismatch

---

**Step 3: Update resolve functions**

Edit `apps/desktop/src-tauri/src/services/shortcut.rs`:

```rust
/// Resolve polish_template_id from a specific profile key.
pub fn resolve_profile_template(profiles: &ShortcutProfilesMap, key: &str) -> Option<String> {
    let profile = match key {
        "dictate" => &profiles.dictate,
        "chat" => &profiles.chat,
        "custom" => profiles.custom.as_ref()?,
        _ => return None,
    };
    
    match &profile.action {
        ShortcutAction::Record { polish_template_id } => polish_template_id.clone(),
    }
}

/// Validate profile update constraints.
pub fn validate_profile_update(key: &str, profile: &ShortcutProfile) -> Result<(), String> {
    match key {
        "dictate" => {
            if profile.action.Record?.polish_template_id.is_some() {
                return Err("cannot_update_dictate_template".to_string());
            }
        }
        "chat" => {
            if profile.action.Record?.polish_template_id.is_none() {
                return Err("chat_template_cannot_be_null".to_string());
            }
        }
        "custom" => {} // No constraints
        _ => return Err("unknown_profile_key".to_string()),
    }
    Ok(())
}

/// Check hotkey uniqueness across all profiles.
pub fn validate_hotkey_unique(profiles: &ShortcutProfilesMap, hotkey: &str, exclude_key: &str) -> Result<(), String> {
    let all_profiles: Vec<(&str, &ShortcutProfile)> = vec![
        ("dictate", &profiles.dictate),
        ("chat", &profiles.chat),
    ].into_iter()
    .chain(profiles.custom.as_ref().map(|p| ("custom", p)))
    .collect();
    
    for (key, profile) in all_profiles {
        if key != exclude_key && profile.hotkey == hotkey {
            return Err(format!("hotkey '{}' already used by profile '{}'", hotkey, key));
        }
    }
    Ok(())
}
```

---

**Step 4: Remove obsolete functions**

Remove:
- `resolve_record_polish_override`
- `resolve_global_polish`
- `find_profile_by_id`
- `find_profile_by_hotkey`

---

**Step 5: Run tests to verify**

Run: `cargo test --lib services::shortcut`

Expected: All tests PASS

---

**Step 6: Commit**

```bash
git add apps/desktop/src-tauri/src/services/shortcut.rs
git commit -m "refactor(services): update resolve/validation for ShortcutProfilesMap"
```

---

### Unit 3: Update PreparedRecordingStart for profile key

**Goal:** Accept profile key instead of full profile, resolve template_id from map.

**Files:**
- Modify: `apps/desktop/src-tauri/src/services/recording_lifecycle.rs`
- Modify: `apps/desktop/src-tauri/src/services/recording_lifecycle.rs` tests

---

**Step 1: Write failing test**

Add to tests:

```rust
#[test]
fn prepare_recording_start_resolves_from_profile_key() {
    let state = AppState::new();
    let profiles = ShortcutProfilesMap {
        dictate: ShortcutProfile {
            hotkey: "Shift+Space".to_string(),
            action: ShortcutAction::Record { polish_template_id: None },
        },
        chat: ShortcutProfile {
            hotkey: "Cmd+Space".to_string(),
            action: ShortcutAction::Record { polish_template_id: Some("formal".to_string()) },
        },
        custom: None,
    };
    
    let prepared_dictate = prepare_recording_start(&state, &profiles, "dictate");
    assert_eq!(prepared_dictate.resolved_polish_template_id, None);
    
    let prepared_chat = prepare_recording_start(&state, &profiles, "chat");
    assert_eq!(prepared_chat.resolved_polish_template_id, Some("formal".to_string()));
}
```

---

**Step 2: Run test to verify failure**

Run: `cargo test --lib services::recording_lifecycle::prepare_recording_start_resolves_from_profile_key`

Expected: FAIL — signature mismatch

---

**Step 3: Modify function signature**

Edit `apps/desktop/src-tauri/src/services/recording_lifecycle.rs`:

```rust
pub fn prepare_recording_start(
    state: &AppState,
    profiles: &ShortcutProfilesMap,
    profile_key: &str,
) -> PreparedRecordingStart {
    let (cloud_stt_enabled, cloud_stt_config, language) = {
        let settings = state.settings.lock();
        (
            #[allow(deprecated)]
            settings.is_volcengine_streaming_active(),
            settings.get_active_cloud_stt_config(),
            settings.stt_engine_language.clone(),
        )
    };

    let resolved_polish_template_id = 
        crate::services::shortcut::resolve_profile_template(profiles, profile_key);

    let task_id = allocate_task_id(state);
    state.start_session(task_id);

    // ... rest unchanged ...

    PreparedRecordingStart {
        task_id,
        cloud_stt_enabled,
        cloud_stt_config,
        language,
        resolved_polish_template_id,
    }
}
```

---

**Step 4: Update caller in shortcut manager**

Pass profile key instead of profile reference when triggering recording.

---

**Step 5: Run tests to verify**

Run: `cargo test --lib services::recording_lifecycle`

Expected: All tests PASS

---

**Step 6: Commit**

```bash
git add apps/desktop/src-tauri/src/services/recording_lifecycle.rs
git commit -m "refactor(recording): use profile_key to resolve template from map"
```

---

### Unit 4: Update polish execution to use template_id

**Goal:** Polish execution reads template_id from prepared, gets prompt from template, uses global provider/model.

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/audio/polish.rs`
- Modify: `apps/desktop/src-tauri/src/polish_engine/templates.rs` (add helper if needed)

---

**Step 1: Update maybe_polish_transcription_text**

Edit `apps/desktop/src-tauri/src/commands/audio/polish.rs`:

Replace the function signature and logic:

```rust
pub(super) async fn maybe_polish_transcription_text(
    app: &AppHandle,
    task_id: u64,
    raw_text: String,
    resolved_polish_template_id: Option<String>,
) -> String {
    use crate::polish_engine::{get_template_by_id, DEFAULT_POLISH_PROMPT};

    let state = app
        .try_state::<crate::state::app_state::AppState>()
        .expect("AppState required");

    match resolved_polish_template_id {
        None => {
            info!(task_id, "polish_skipped-no_template");
            raw_text
        }
        Some(template_id) => {
            let settings = state.settings.lock();
            
            // Get template's system_prompt
            let system_prompt = get_template_by_id(&template_id)
                .map(|t| t.system_prompt)
                .or_else(|| {
                    settings.polish_custom_templates
                        .iter()
                        .find(|t| t.id == template_id)
                        .map(|t| t.system_prompt.clone())
                })
                .unwrap_or_else(|| {
                    warn!(task_id, template_id = %template_id, "template_not_found_fallback");
                    get_template_by_id("filler")
                        .map(|t| t.system_prompt)
                        .unwrap_or(DEFAULT_POLISH_PROMPT)
                });

            // Provider resolution: cloud first, then local fallback
            let provider_type = settings.active_cloud_polish_provider.clone();
            let cloud_config = settings.cloud_polish_configs.get(&provider_type);
            let polish_model = settings.polish_model.clone();
            drop(settings);

            // Try cloud first if configured
            if let Some(config) = cloud_config {
                if !config.api_key.is_empty() && !config.model.is_empty() {
                    info!(task_id, provider = %provider_type, model = %config.model, "polish_started-cloud");

                    let request = crate::polish_engine::PolishRequest::new(
                        raw_text.clone(),
                        system_prompt,
                        None,
                    );

                    event_target.emit_polishing(task_id);

                    match state.polish_manager.polish_cloud(
                        request,
                        &provider_type,
                        &config.api_key,
                        &config.base_url,
                        &config.model,
                        config.enable_thinking,
                    ).await {
                        Ok(result) if !result.text.is_empty() => {
                            info!(task_id, polish_ms = result.total_ms, "polish_completed-cloud");
                            return result.text;
                        }
                        Ok(_) => {
                            warn!(task_id, "polish_empty_result-cloud");
                        }
                        Err(e) => {
                            warn!(task_id, error = %e, "polish_failed-cloud");
                        }
                    }
                }
            }

            // Fallback to local polish
            if !polish_model.is_empty() {
                return run_local_polish(
                    app,
                    task_id,
                    raw_text,
                    system_prompt,
                    polish_model,
                ).await;
            }

            // No provider configured
            warn!(task_id, "polish_skipped-no_provider_configured");
            raw_text
        }
    }
}
```

---

**Step 2: Update run_local_polish signature**

Change to accept `system_prompt` and `polish_model` directly:

```rust
async fn run_local_polish(
    app: &AppHandle,
    task_id: u64,
    raw_text: String,
    system_prompt: String,
    polish_model_id: String,
) -> String {
    let state = app
        .try_state::<crate::state::app_state::AppState>()
        .expect("AppState required");

    match crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id) {
        Some(engine_type) => {
            if state.polish_manager.is_model_loaded(&engine_type, &polish_model_id) {
                info!(task_id, engine = ?engine_type, model_id = %polish_model_id, "polish_started-local");

                let request = crate::polish_engine::PolishRequest::new(
                    raw_text.clone(),
                    system_prompt,
                    None,
                );

                event_target.emit_polishing(task_id);

                match state.polish_manager.polish(engine_type, request).await {
                    Ok(result) if !result.text.is_empty() => {
                        info!(task_id, polish_ms = result.total_ms, "polish_completed-local");
                        result.text
                    }
                    Ok(_) => {
                        warn!(task_id, "polish_empty_result-local");
                        raw_text
                    }
                    Err(e) => {
                        warn!(task_id, error = %e, "polish_failed-local");
                        raw_text
                    }
                }
            } else {
                warn!(task_id, model_id = %polish_model_id, "polish_model_not_downloaded");
                raw_text
            }
        }
        None => {
            warn!(task_id, model_id = %polish_model_id, "polish_model_unknown");
            raw_text
        }
    }
}
```

---

**Step 3: Export get_template_by_id from templates.rs**

Ensure `polish_engine/templates.rs` exports `get_template_by_id`:

```rust
pub fn get_template_by_id(id: &str) -> Option<&'static PolishTemplate> {
    POLISH_TEMPLATES.iter().find(|t| t.id == id)
}
```

Ensure `polish_engine/mod.rs` exports:

```rust
pub use templates::{get_template_by_id, POLISH_TEMPLATES, DEFAULT_POLISH_PROMPT};
```

Add `DEFAULT_POLISH_PROMPT` constant if not present:

```rust
pub const DEFAULT_POLISH_PROMPT: &str = "Remove filler words. Keep the same language as input.";
```

---

**Step 4: Update caller in capture.rs**

Find where `maybe_polish_transcription_text` is called and update to pass `resolved_polish_template_id`:

Search for calls and update signature usage.

---

**Step 5: Run cargo check**

Run: `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: No errors

---

**Step 6: Run tests**

Run: `cargo test --lib commands::audio::polish`

Expected: Tests pass (may need to update test signatures)

---

**Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/commands/audio/polish.rs
git add apps/desktop/src-tauri/src/polish_engine/templates.rs
git add apps/desktop/src-tauri/src/polish_engine/mod.rs
git commit -m "refactor(polish): use template_id for system_prompt, global provider/model"
```

---

### Unit 5: Remove obsolete polish settings fields

**Goal:** Remove `polish_enabled`, `polish_selected_template`, `cloud_polish_enabled` from AppSettings struct.

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/settings/mod.rs`

---

**Step 1: Update AppSettings struct**

Edit `apps/desktop/src-tauri/src/commands/settings/mod.rs`:

```rust
pub struct AppSettings {
    // ... other fields unchanged ...
    
    // NEW: shortcut profiles map
    pub shortcut_profiles: ShortcutProfilesMap,
    
    // REMOVED:
    // pub polish_enabled: bool,
    // pub polish_selected_template: String,
    // pub cloud_polish_enabled: bool,
    
    // UNCHANGED: polish provider settings
    pub polish_model: String,
    pub active_cloud_polish_provider: String,
    pub cloud_polish_configs: HashMap<String, CloudProviderConfig>,
    pub polish_custom_templates: Vec<CustomPolishTemplate>,
}
```

---

**Step 2: Update Default impl**

```rust
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shortcut_profiles: ShortcutProfilesMap::default(),
            polish_model: String::new(),
            active_cloud_polish_provider: String::new(),
            cloud_polish_configs: HashMap::new(),
            polish_custom_templates: Vec::new(),
            // ... other defaults ...
        }
    }
}
```

---

**Step 3: Update any code reading obsolete fields**

Run: `grep -rE "polish_enabled|polish_selected_template|cloud_polish_enabled" apps/desktop/src-tauri/src/`

Remove all usages.

---

**Step 4: Run cargo check**

Run: `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: No errors

---

**Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/commands/settings/mod.rs
git commit -m "refactor(settings): remove polish_enabled, polish_selected_template, cloud_polish_enabled; add ShortcutProfilesMap"
```

Remove from `Default` impl.

Remove from JSON field parsing if explicit.

---

**Step 2: Update settings migration**

Add migration logic to handle old format:

```rust
fn migrate_obsolete_polish_fields(json: &mut serde_json::Value) {
    if let Some(obj) = json.as_object_mut() {
        obj.remove("polish_enabled");
        obj.remove("polish_selected_template");
        obj.remove("cloud_polish_enabled");
    }
}
```

Call this during settings load.

---

**Step 3: Update any code reading obsolete fields**

Search for usage and remove:

Run: `grep -r "polish_enabled\|polish_selected_template\|cloud_polish_enabled" apps/desktop/src-tauri/src/`

For any matches:
- `polish_enabled` → remove (now per-profile via template_id)
- `polish_selected_template` → remove (each profile has own template_id)
- `cloud_polish_enabled` → remove (implicit from active_cloud_polish_provider + cloud_polish_configs)

---

**Step 4: Run cargo check**

Run: `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: No errors

---

**Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/commands/settings/mod.rs
git commit -m "refactor(settings): remove polish_enabled, polish_selected_template, cloud_polish_enabled"
```

---

### Unit 6: Update frontend types for map structure

**Goal:** Update TypeScript types for ShortcutProfilesMap.

**Files:**
- Modify: `apps/desktop/src/lib/tauri.ts`

---

**Step 1: Define ShortcutProfilesMap interface**

Edit `apps/desktop/src/lib/tauri.ts`:

```typescript
export interface ShortcutProfilesMap {
  dictate: ShortcutProfile;
  chat: ShortcutProfile;
  custom?: ShortcutProfile;
}

export interface ShortcutProfile {
  hotkey: string;
  action: {
    Record?: {
      polish_template_id?: string | null;
    };
  };
}
```

---

**Step 2: Update AppSettings interface**

```typescript
export interface AppSettings {
  // ... other fields ...
  shortcut_profiles: ShortcutProfilesMap;  // Changed from array to map
  // REMOVED: polish_enabled
  // REMOVED: polish_selected_template
  // REMOVED: cloud_polish_enabled
  polish_model: string;
  active_cloud_polish_provider: string;
  cloud_polish_configs: Record<string, CloudProviderConfig>;
}
```

---

**Step 3: Update command signatures**

```typescript
export const shortcutCommands = {
  getProfiles: () => invokeWithLogging<ShortcutProfilesMap>("get_shortcut_profiles"),
  updateProfile: (key: string, profile: ShortcutProfile) =>
    invokeWithLogging<void>("update_shortcut_profile", { key, profile }),
  createCustom: (profile: ShortcutProfile) =>
    invokeWithLogging<void>("create_custom_profile", { profile }),
  deleteCustom: () =>
    invokeWithLogging<void>("delete_custom_profile"),
};
```

---

**Step 4: Run TypeScript check**

Run: `pnpm --filter @ariatype/desktop build`

Expected: No TypeScript errors

---

**Step 5: Commit**

```bash
git add apps/desktop/src/lib/tauri.ts
git commit -m "refactor(frontend): ShortcutProfilesMap {dictate, chat, custom}"
```

---

### Unit 7: Create frontend profile UI for map structure

**Goal:** Render three fixed profile sections (dictate/chat/custom) with constraints.

**Files:**
- Modify: `apps/desktop/src/components/Home/HotkeySettings.tsx`

---

**Step 1: Render three profile sections**

Edit `apps/desktop/src/components/Home/HotkeySettings.tsx`:

```typescript
export function HotkeySettings() {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettingsContext();
  const [templates, setTemplates] = useState<PolishTemplate[]>([]);
  const profiles = settings?.shortcut_profiles;

  useEffect(() => {
    loadTemplates();
  }, []);

  const loadTemplates = async () => {
    const [builtIn, custom] = await Promise.all([
      modelCommands.getPolishTemplates(),
      modelCommands.getPolishCustomTemplates(),
    ]);
    setTemplates([...builtIn, ...custom]);
  };

  return (
    <SettingsPageLayout title={t("hotkey.title")} testId="hotkey-page">
      <Card>
        <CardContent className="space-y-6">
          {/* Dictate - fixed template */}
          <ProfileSection
            label="Dictate"
            profile={profiles?.dictate}
            templates={templates}
            canChangeTemplate={false}
            canDelete={false}
            onUpdate={handleUpdateDictate}
          />

          {/* Chat - template cannot be null */}
          <ProfileSection
            label="Chat"
            profile={profiles?.chat}
            templates={templates}
            canChangeTemplate={true}
            allowNullTemplate={false}
            canDelete={false}
            onUpdate={handleUpdateChat}
          />

          {/* Custom - optional */}
          {profiles?.custom ? (
            <ProfileSection
              label="Custom"
              profile={profiles.custom}
              templates={templates}
              canChangeTemplate={true}
              allowNullTemplate={true}
              canDelete={true}
              onUpdate={handleUpdateCustom}
              onDelete={handleDeleteCustom}
            />
          ) : (
            <Button onClick={handleCreateCustom}>
              <Plus className="h-4 w-4 mr-2" />
              Create Custom Profile
            </Button>
          )}
        </CardContent>
      </Card>

      {/* Recording Mode */}
      <Card className="mt-4">
        <CardContent>
          <MultiSwitch
            options={recordingModes}
            value={settings?.recording_mode || "hold"}
            onChange={saveRecordingMode}
          />
        </CardContent>
      </Card>
    </SettingsPageLayout>
  );
}
```

---

**Step 2: Create ProfileSection component**

```typescript
interface ProfileSectionProps {
  label: string;
  profile?: ShortcutProfile;
  templates: PolishTemplate[];
  canChangeTemplate: boolean;
  allowNullTemplate: boolean;
  canDelete: boolean;
  onUpdate: (hotkey: string, templateId: string | null) => void;
  onDelete?: () => void;
}

function ProfileSection({
  label,
  profile,
  templates,
  canChangeTemplate,
  allowNullTemplate,
  canDelete,
  onUpdate,
  onDelete,
}: ProfileSectionProps) {
  const { t } = useTranslation();
  const templateId = profile?.action.Record?.polish_template_id;

  return (
    <div className="p-4 rounded-2xl border space-y-3">
      <div className="flex justify-between items-center">
        <div className="font-medium">{label}</div>
        {canDelete && (
          <Button variant="ghost" size="icon" onClick={onDelete}>
            <Trash2 className="h-4 w-4" />
          </Button>
        )}
      </div>

      <div className="space-y-2">
        <Label>{t("hotkey.hotkey")}</Label>
        <HotkeyInput
          value={profile?.hotkey || ""}
          onChange={(h) => onUpdate(h, templateId)}
        />
      </div>

      <div className="space-y-2">
        <Label>{t("hotkey.template")}</Label>
        {canChangeTemplate ? (
          <Select
            value={templateId || ""}
            onValueChange={(v) => onUpdate(profile?.hotkey || "", allowNullTemplate ? (v || null) : v)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {allowNullTemplate && (
                <SelectItem value="">{t("hotkey.noPolish")}</SelectItem>
              )}
              {templates.map((t) => (
                <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <div className="text-muted-foreground">{t("hotkey.noPolish")}</div>
        )}
      </div>
    </div>
  );
}
```

---

**Step 3: Add handlers**

```typescript
const handleUpdateDictate = async (hotkey: string, _: string | null) => {
  // template_id always null for dictate
  await shortcutCommands.updateProfile("dictate", {
    hotkey,
    action: { Record: { polish_template_id: null } },
  });
};

const handleUpdateChat = async (hotkey: string, templateId: string | null) => {
  if (!templateId) {
    toast.error(t("hotkey.chatTemplateRequired"));
    return;
  }
  await shortcutCommands.updateProfile("chat", {
    hotkey,
    action: { Record: { polish_template_id: templateId } },
  });
};

const handleUpdateCustom = async (hotkey: string, templateId: string | null) => {
  await shortcutCommands.updateProfile("custom", {
    hotkey,
    action: { Record: { polish_template_id: templateId } },
  });
};

const handleCreateCustom = async () => {
  await shortcutCommands.createCustom({
    hotkey: "",
    action: { Record: { polish_template_id: templates[0]?.id } },
  });
};

const handleDeleteCustom = async () => {
  await shortcutCommands.deleteCustom();
};
```

---

**Step 4: Run frontend build**

Run: `pnpm --filter @ariatype/desktop build`

Expected: Build succeeds

---

**Step 5: Commit**

```bash
git add apps/desktop/src/components/Home/HotkeySettings.tsx
git commit -m "feat(ui): render dictate/chat/custom profile sections with constraints"
```

---

### Unit 8: Settings migration from array/hotkey to map

**Goal:** Migrate old settings (array profiles or single hotkey) to ShortcutProfilesMap.

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/settings/mod.rs`

---

**Step 1: Write migration logic**

```rust
fn migrate_to_profiles_map(json: &mut serde_json::Value) -> Result<(), String> {
    // Remove obsolete fields
    if let Some(obj) = json.as_object_mut() {
        obj.remove("hotkey");
        obj.remove("polish_enabled");
        obj.remove("polish_selected_template");
        obj.remove("cloud_polish_enabled");
        
        // Check if shortcut_profiles is already a map
        if let Some(profiles) = obj.get("shortcut_profiles") {
            if profiles.is_object() {
                return Ok(()); // Already migrated
            }
            
            // If array, convert to map
            if let Some(arr) = profiles.as_array() {
                let mut map = serde_json::Map::new();
                
                // First element → dictate
                if let Some(first) = arr.first() {
                    map.insert("dictate".to_string(), first.clone());
                } else {
                    map.insert("dictate".to_string(), serde_json::json!({
                        "hotkey": "Shift+Space",
                        "action": { "Record": { "polish_template_id": null } }
                    }));
                }
                
                // Second element → chat
                if let Some(second) = arr.get(1) {
                    map.insert("chat".to_string(), second.clone());
                } else {
                    map.insert("chat".to_string(), serde_json::json!({
                        "hotkey": "",
                        "action": { "Record": { "polish_template_id": "filler" } }
                    }));
                }
                
                // Third element → custom (if exists)
                if let Some(third) = arr.get(2) {
                    map.insert("custom".to_string(), third.clone());
                }
                
                obj.insert("shortcut_profiles".to_string(), serde_json::Value::Object(map));
            }
        } else {
            // No shortcut_profiles, create from old hotkey
            let old_hotkey = obj.get("hotkey")
                .and_then(|v| v.as_str())
                .unwrap_or("Shift+Space");
            
            obj.insert("shortcut_profiles".to_string(), serde_json::json!({
                "dictate": {
                    "hotkey": old_hotkey,
                    "action": { "Record": { "polish_template_id": null } }
                },
                "chat": {
                    "hotkey": "",
                    "action": { "Record": { "polish_template_id": "filler" } }
                }
            }));
        }
    }
    Ok(())
}
```

---

**Step 2: Call migration during settings load**

Call `migrate_to_profiles_map` in the settings loading path.

---

**Step 3: Run cargo check**

Run: `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`

Expected: No errors

---

**Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/commands/settings/mod.rs
git commit -m "feat(settings): migrate array/hotkey to ShortcutProfilesMap"
```

---

## Verification Evidence

### Backend

```bash
cargo test --lib shortcut::profile_types
cargo test --lib services::shortcut
cargo test --lib services::recording_lifecycle
cargo test --lib commands::audio::polish
cargo clippy --all-features -- -D warnings
cargo fmt -- --check
```

### Frontend

```bash
pnpm --filter @ariatype/desktop build
pnpm --filter @ariatype/shared typecheck
pnpm check:i18n
```

### Manual

1. Start app → verify profiles map: dictate (hotkey set), chat (hotkey empty), no custom
2. Configure chat hotkey → verify registered
3. Press dictate hotkey → verify raw STT (no polish)
4. Press chat hotkey → verify polish with default template
5. Change chat template → verify new template used
6. Attempt to set dictate template → verify rejection
7. Attempt to set chat template to None → verify rejection
8. Create custom profile → verify created
9. Delete custom profile → verify removed
10. Attempt to delete dictate/chat → verify rejection
11. Remove cloud config → verify fallback to local polish
12. Remove all polish configs → verify polish skipped with warning

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Old settings with array profiles | Migration converts array → map (first→dictate, second→chat, third→custom) |
| Old settings with single hotkey | Migration creates map from hotkey field |
| Template deleted while profile references it | Fallback to first built-in template with warning |
| User tries to set dictate template | Backend rejects with `cannot_update_dictate_template` |
| User tries to set chat template to null | Backend rejects with `chat_template_cannot_be_null` |
| User tries to delete system profiles | Backend rejects with `cannot_delete_system_profile` |
| User tries to create second custom | Backend rejects with `custom_profile_already_exists` |
| Hotkey conflict across profiles | Validation checks all three profiles before update |

---

## Sources & References

- **Spec document:** [context/brainstorms/2026-04-21-polish-template-shortcut-profiles.md](../../brainstorms/2026-04-21-polish-template-shortcut-profiles.md)
- **Multi-shortcut spec:** [context/feat/multi-shortcut/1.0.0/prd/erd.md](../../feat/multi-shortcut/1.0.0/prd/erd.md)
- **Polish templates:** [apps/desktop/src-tauri/src/polish_engine/templates.rs](../../../apps/desktop/src-tauri/src/polish_engine/templates.rs)