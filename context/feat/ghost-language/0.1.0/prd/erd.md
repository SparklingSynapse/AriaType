# Ghost-Language Feature Specification

## Version

- Feature: `ghost-language`
- Version: `0.1.0`
- Status: Draft (deferred)

## Overview

Ghost-Language is one of two Ghost modules. It learns the user's language habits—vocabulary, expressions, style preferences—and applies personalization to STT correction and Polish refinement, making outputs more accurate and personalized over time.

See also:
- **Ghost-Action** (`context/feat/ghost-action/0.1.0/prd/erd.md`) — macOS computer-use via Ghost OS MCP server

## Problem

Current STT and Polish outputs are generic. They don't adapt to the user's unique language patterns:

1. **STT misrecognitions**: Same errors repeat (e.g., "write code" → "right code", "fix bug" → "fix bog")
2. **Polish generic style**: Outputs don't match user's preferred tone, terminology, or formatting
3. **No learning loop**: Corrections are one-time; the system doesn't remember and improve

Users who dictate regularly accumulate patterns: preferred vocabulary, common phrase structures, domain-specific terminology. Without learning, each transcription starts from zero.

## Goal

Build a language learning system that:
1. Observes user corrections and Polish feedback
2. Accumulates language patterns over time
3. Applies corrections to future STT outputs
4. Personalizes Polish prompts with user style
5. Makes outputs more accurate the longer the user uses the app

## First-Principles Model

Ghost-Language must answer three questions:

1. **What to learn?** — Corrections (original → edited), vocabulary preferences, style patterns
2. **When to apply?** — After STT, before Polish; during Polish prompt construction
3. **How to improve?** — Accumulate data, weight by frequency, decay stale patterns

Key insight: Learning is passive observation. User makes corrections → system records → future outputs benefit. No active training required.

## Information Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                     AriaType App (Tauri v2)                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     Frontend (React)                         │   │
│  │   - Ghost learning progress indicator                        │   │
│  │   - "Edit & teach Ghost" mode after transcription            │   │
│  │   - Language profile viewer (vocabulary, patterns)           │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │ IPC                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     Backend (Rust)                           │   │
│  │  ┌───────────────────────────────────────────────────────┐  │   │
│  │  │ ghost/language/                                        │  │   │
│  │  │   ├── types.rs      — LanguageProfile, CorrectionPair  │  │   │
│  │  │   ├── learner.rs    — Observation, accumulation logic   │  │   │
│  │  │   ├── adapter.rs    — STT correction, Polish prompt     │  │   │
│  │  │   ├── storage.rs    — Persistent profile storage        │  │   │
│  │  │   └── stats.rs      — Learning statistics, decay        │  │   │
│  │  └───────────────────────────────────────────────────────┘  │   │
│  │                                                              │   │
│  │  Learning Sources:                                           │   │
│  │   - History edits (original → polished → user_edit)          │   │
│  │   - Manual correction pairs (explicit teaching)              │   │
│  │   - Polish feedback (accept/reject)                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Learning Flow

```
Transcription → STT Output → [Ghost-Language Correction Adapter]
                                    │
                                    ▼
                            Corrected STT Output
                                    │
                                    ▼
                            Polish Engine (personalized prompt)
                                    │
                                    ▼
                            Polished Output
                                    │
                                    ▼
                            User Review
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    ▼               ▼               ▼
            Accept as-is       Edit text       Reject polish
                    │               │               │
                    │               ▼               │
                    │       Record correction      │
                    │       (polished → edited)    │
                    │               │               │
                    └───────────────┼───────────────┘
                                    │
                                    ▼
                            Ghost-Language Learner
                                    │
                                    ▼
                            Update LanguageProfile
                                    │
                                    ▼
                            Storage (persistent)
```

### Ghost-Language Module Structure

| Layer | File | Responsibility |
|-------|------|----------------|
| `types/` | `ghost/language/types.rs` | LanguageProfile, CorrectionPair, VocabularyEntry data structures |
| `learner/` | `ghost/language/learner.rs` | Observation, accumulation, frequency weighting, decay logic |
| `adapter/` | `ghost/language/adapter.rs` | STT post-processing correction, Polish prompt personalization |
| `storage/` | `ghost/language/storage.rs` | Profile persistence (SQLite or JSON), migration, backup |
| `stats/` | `ghost/language/stats.rs` | Learning statistics, progress metrics, health checks |
| `command/` | `commands/ghost_language.rs` | Tauri IPC handlers (ghost_language_status, ghost_language_profile) |

### Layer Compliance

The Ghost-Language module follows the backend layer model from `architecture/layers.md`:

```
ghost/language/types.rs     — pure data structures
        ↓
ghost/language/storage.rs   — profile persistence, SQLite/JSON
        ↓
ghost/language/learner.rs   — observation, accumulation logic
        ↓
ghost/language/stats.rs     — statistics, progress metrics
        ↓
ghost/language/adapter.rs   — STT correction, Polish prompt
        ↓
commands/ghost_language.rs  — Tauri IPC adapters
        ↓
events/ghost_language.rs    — backend → frontend events
```

Boundary rule enforcement:
- `ghost/language/adapter.rs` may apply corrections but NOT emit Tauri events
- `commands/ghost_language.rs` receives learner results and emits events via Tauri handle
- No `ghost/language/` → `commands/` reverse dependency

## Data Contract

### LanguageProfile

```typescript
interface LanguageProfile {
  user_id: string;              // Unique profile identifier
  created_at: number;           // Profile creation timestamp
  updated_at: number;           // Last update timestamp
  vocabulary: VocabularyEntry[];
  correction_pairs: CorrectionPair[];
  style_patterns: StylePattern[];
  stats: LanguageStats;
}
```

### VocabularyEntry

```typescript
interface VocabularyEntry {
  term: string;                 // User's preferred term (e.g., "React hooks")
  aliases: string[];            // Terms that should map to this (e.g., ["react hook", "hooks"])
  frequency: number;            // Usage count
  last_used: number;            // Timestamp of last usage
  domain: string;               // Context domain (e.g., "programming", "email", "general")
  decay_weight: number;         // Current weight after decay (0.0-1.0)
}
```

### CorrectionPair

```typescript
interface CorrectionPair {
  original: string;             // STT original output (e.g., "right code")
  corrected: string;            // User's correction (e.g., "write code")
  frequency: number;            // How many times this correction was made
  confidence: number;           // Learning confidence (0.0-1.0)
  last_applied: number;         // Timestamp when last applied successfully
  source: "history_edit" | "explicit_teach" | "polish_feedback";
}
```

### StylePattern

```typescript
interface StylePattern {
  category: "tone" | "formatting" | "structure";
  pattern: string;              // Pattern description
  examples: string[];           // Example outputs
  frequency: number;
}
```

### LanguageStats

```typescript
interface LanguageStats {
  total_corrections: number;    // Total correction pairs learned
  total_vocab_entries: number;  // Total vocabulary entries
  successful_applications: number; // Times corrections were applied
  failed_applications: number;  // Times corrections were rejected
  learning_days: number;        // Days since first learning
  last_learning: number;        // Timestamp of last learning event
}
```

### GhostLanguageStatus

```typescript
interface GhostLanguageStatus {
  enabled: boolean;             // Feature enabled in settings
  profile_exists: boolean;      // Language profile created
  learning_active: boolean;     // Currently accumulating data
  stats: LanguageStats;
  progress: {
    corrections_learned: number;
    vocabulary_learned: number;
    style_patterns_learned: number;
  };
}
```

### Backend Events

| Event | Payload | Trigger |
|-------|---------|---------|
| `ghost-language-status-changed` | `GhostLanguageStatus` | Feature enabled/disabled, profile created |
| `ghost-language-correction-learned` | `{ original, corrected, source }` | New correction pair added |
| `ghost-language-vocabulary-learned` | `{ term, aliases }` | New vocabulary entry added |
| `ghost-language-correction-applied` | `{ original, corrected, context }` | Correction applied to STT output |
| `ghost-language-progress` | `{ stats }` | Learning progress update |

## Acceptance Criteria

1. **Profile Creation**: On first transcription with Ghost-Language enabled, backend creates a LanguageProfile.
2. **Correction Learning**: When user edits transcription in history, backend records correction pair (polished → edited).
3. **Vocabulary Learning**: Backend extracts vocabulary from frequently used terms in user history.
4. **STT Correction Adapter**: After STT output, backend applies high-confidence correction pairs.
5. **Polish Prompt Personalization**: Backend injects user style patterns into Polish prompts.
6. **Frequency Weighting**: Corrections weighted by frequency; high-frequency corrections have higher confidence.
7. **Decay Mechanism**: Stale corrections (not applied in 30 days) decay in confidence.
8. **Persistence**: LanguageProfile stored persistently; survives app restarts.
9. **Cross-Language Support**: Learning works for all supported languages (en, zh, de, es, fr, it, ja, ko, pt, ru).
10. **Privacy**: Profile stored locally; no cloud sync by default.
11. **Settings Integration**: Ghost-Language toggle in settings; reset profile option.
12. **Progress Display**: Frontend shows learning progress (corrections, vocabulary count).
13. **i18n**: All user-facing Ghost-Language UI copy is internationalized.

## BDD Scenarios

### Scenario: First-time Ghost-Language user

- Given Ghost-Language is enabled in settings
- And user has no existing LanguageProfile
- When first transcription completes
- Then backend creates empty LanguageProfile
- And frontend shows "Ghost is learning your language" message

### Scenario: Learn correction from history edit

- Given user has transcription history entry with polished text "write code"
- And user edits it to "Write code in React hooks style"
- When user saves the edit
- Then backend records CorrectionPair `{ original: "write code", corrected: "Write code in React hooks style" }`
- And emits `ghost-language-correction-learned` event
- And frontend shows "Ghost learned 1 correction"

### Scenario: Apply correction to STT output

- Given LanguageProfile has CorrectionPair `{ original: "right code", corrected: "write code", confidence: 0.9 }`
- When STT outputs "right code today"
- Then Ghost-Language adapter applies correction → "write code today"
- And emits `ghost-language-correction-applied` event

### Scenario: Learn vocabulary from frequent terms

- Given user has 50+ transcription history entries
- And "React hooks" appears in 10+ entries
- When backend runs vocabulary extraction
- Then VocabularyEntry `{ term: "React hooks", aliases: ["react hook"], frequency: 10 }` added
- And emits `ghost-language-vocabulary-learned` event

### Scenario: Decay stale corrections

- Given CorrectionPair `{ original: "old term", corrected: "new term", last_applied: 45 days ago }`
- When backend runs decay maintenance
- Then confidence decays to 0.3 (below threshold)
- And correction is removed from active corrections

### Scenario: Personalize Polish prompt

- Given LanguageProfile has StylePattern `{ category: "tone", pattern: "formal", examples: [...] }`
- When Polish engine processes text
- Then prompt includes "Maintain formal tone per user preference"
- And output reflects personalized style

### Scenario: Reset LanguageProfile

- Given user has accumulated LanguageProfile with 100 corrections
- When user clicks "Reset Ghost Language" in settings
- Then backend deletes LanguageProfile
- And creates fresh empty profile
- And frontend shows "Ghost language reset"

## Verification

### Backend Tests

```bash
# Rust unit tests for ghost-language module
cargo test ghost::language:: --no-fail-fast

# Learner logic tests
cargo test language_learner::

# Adapter tests
cargo test language_adapter::

# Storage tests
cargo test language_storage::

# Decay tests
cargo test language_decay::
```

### Frontend Tests

```bash
# Component tests for Ghost-Language UI
pnpm --filter @ariatype/desktop test ghost-language

# Build verification
pnpm --filter @ariatype/desktop build
```

### Manual Verification

1. Enable Ghost-Language in settings
2. Transcribe text, edit in history, verify correction learned
3. Transcribe similar text, verify correction applied
4. Check LanguageProfile viewer shows accumulated data
5. Reset profile, verify data cleared

## Implementation Notes

### Correction Adapter Design

```rust
// ghost/language/adapter.rs

pub struct LanguageAdapter {
    profile: LanguageProfile,
}

impl LanguageAdapter {
    pub fn correct_stt_output(&self, text: &str) -> String {
        let mut corrected = text.to_string();
        
        for pair in &self.profile.correction_pairs {
            if pair.confidence > 0.7 {
                corrected = corrected.replace(&pair.original, &pair.corrected);
            }
        }
        
        corrected
    }
    
    pub fn personalize_polish_prompt(&self, base_prompt: &str) -> String {
        let style_hints = self.extract_style_hints();
        format!("{} User prefers: {}", base_prompt, style_hints)
    }
}
```

### Learner Design

```rust
// ghost/language/learner.rs

pub struct LanguageLearner {
    profile: LanguageProfile,
    storage: LanguageStorage,
}

impl LanguageLearner {
    pub fn observe_correction(&mut self, original: &str, corrected: &str, source: CorrectionSource) {
        // Find existing pair or create new
        let pair = self.profile.correction_pairs.iter_mut()
            .find(|p| p.original == original)
            .unwrap_or_else(|| {
                self.profile.correction_pairs.push(CorrectionPair::new(original, corrected, source));
                self.profile.correction_pairs.last_mut().unwrap()
            });
        
        pair.frequency += 1;
        pair.confidence = self.calculate_confidence(pair.frequency);
        pair.last_applied = now();
        
        self.storage.save(&self.profile);
    }
    
    fn calculate_confidence(&self, frequency: u32) -> f32 {
        // Confidence grows with frequency, capped at 1.0
        min(1.0, frequency as f32 / 10.0)
    }
}
```

### Decay Mechanism

```rust
// ghost/language/stats.rs

pub fn apply_decay(profile: &mut LanguageProfile, now: i64) {
    let decay_threshold_days = 30;
    
    for pair in &mut profile.correction_pairs {
        let days_since_applied = (now - pair.last_applied) / 86400;
        if days_since_applied > decay_threshold_days {
            pair.confidence *= 0.9.powi(days_since_applied as i32 - decay_threshold_days);
        }
    }
    
    // Remove pairs below 0.3 confidence
    profile.correction_pairs.retain(|p| p.confidence > 0.3);
}
```

### Settings Schema Addition

```typescript
// Settings type extension
interface Settings {
  // ... existing fields
  ghost_language: {
    enabled: boolean;           // Ghost-Language feature on/off
    auto_learn: boolean;        // Automatically learn from history edits
    decay_days: number;         // Days before decay kicks in (default 30)
    confidence_threshold: number; // Minimum confidence to apply correction (default 0.7)
  };
}
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Over-correction (too many replacements) | Confidence threshold; frequency cap; user review |
| Wrong corrections (false positives) | Decay mechanism; frequency weighting; manual removal |
| Privacy concerns (local profile) | Local storage only; no cloud sync; clear reset option |
| Cross-language confusion | Separate profiles per language; language detection |
| Performance overhead | Lazy loading; batch updates; background decay |
| Storage growth | Decay removes stale entries; size limit per profile |

## Dependencies

### External

None (fully local)

### Internal

- `history/` — Source of corrections (original → edited pairs)
- `stt_engine/` — Output correction point
- `polish_engine/` — Prompt personalization point
- `state/unified_state.rs` — LanguageProfile state integration
- `events/` — Ghost-Language event emission
- `commands/` — IPC handler registration

## Platform Support

Ghost-Language works on all platforms (macOS, Windows, Linux). No external dependencies.

## Out of Scope (v0.1.0)

- Cloud sync for LanguageProfile (future feature)
- Vocabulary extraction from external sources (email, documents)
- Active vocabulary training UI (explicit teaching mode)
- Style pattern learning from Polish accept/reject feedback
- Language profile export/import

## Future Enhancements

1. **Cloud Sync** — Sync LanguageProfile across devices
2. **Vocabulary Import** — Import from user documents, email corpus
3. **Explicit Teaching Mode** — UI for manually adding corrections
4. **Polish Feedback Learning** — Learn from accept/reject patterns
5. **Domain Profiles** — Separate profiles per domain (work, personal)
6. **Language Profile Sharing** — Export/import profiles for team sharing