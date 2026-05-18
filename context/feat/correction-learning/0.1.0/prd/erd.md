# Correction Learning Feature Specification

## Version

- Feature: `correction-learning`
- User-visible name: Correction Memory / 纠错记忆
- Version: `0.1.0`
- Status: Active

## Overview

Correction Learning passively learns user correction behavior after AriaType
delivers text. When the user edits delivered text in the target app, AriaType
observes a small before/after text delta and stores a local correction mapping:

```
wrong text -> corrected text
```

The learned mapping is then available to all AriaType variants that run on the
same machine, including development, in-house, and production builds.

## Problem Statement

STT and polish output can still contain product names, domain terms, or personal
wording that the user knows how to correct. Today those edits disappear after
the target application accepts the text, so AriaType cannot improve from the
user's repeated corrections.

## Goals

1. Learn correction pairs, not just corrected terms.
2. Keep the feature local-only and privacy-preserving.
3. Avoid disrupting the recording and text-delivery path.
4. Show a small pill-window tooltip when a new correction is learned.
5. Make learned corrections available to future STT/polish output.
6. Give users control to disable and clear local correction memory.

## Non-Goals

1. Do not record full user documents or editor buffers.
2. Do not require cloud sync or model training.
3. Do not block transcription delivery if edit observation fails.
4. Do not implement a full cross-app editor integration in v0.1.0.

## First-Principles Model

1. The useful learning unit is a bounded edit pair: `wrong -> corrected`.
2. The original and corrected full documents are not the product data model.
3. Learning must happen after delivery, because the target editor is where the
   user confirms the real desired wording.
4. Applying learned mappings before polish gives both deterministic correction
   and model-assisted cleanup a better starting point.
5. One observation is evidence for recording; repeated observations are evidence
   for automatic application.

## Information Architecture

| Surface | User Meaning |
|---------|--------------|
| Target editor | User corrects delivered text naturally where they work |
| Shared correction file | AriaType remembers compact correction pairs locally |
| Pill tooltip | User receives a small confirmation that the correction was learned |

## Architecture

### Backend Module

`apps/desktop/src-tauri/src/correction_learning/`

| File | Responsibility |
|------|----------------|
| `types.rs` | Shared data structures and event payloads |
| `diff.rs` | Extract safe correction pairs from before/after text |
| `storage.rs` | Shared local JSON persistence and correction application |
| `observer.rs` | Short-lived post-delivery edit observation |
| `platform.rs` | Platform adapter for reading focused editable text |

### Shared Storage

Correction data is stored outside the bundle-specific app data directory:

```
<user data dir>/AriaType/correction-learning/corrections.json
```

This path is intentionally shared across dev, in-house, and production builds.
Only compact mappings are stored, not full documents.

Reads and writes use a lightweight lock file beside the correction file to avoid
same-device multi-process writes from corrupting the JSON payload.

### Runtime Flow

```
Recording finishes
  -> final text is emitted and delivered
  -> correction observer starts in the background
  -> observer reads focused editable text via platform adapter
  -> user edits the delivered text
  -> stable before/after delta is converted to wrong -> corrected
  -> mapping is upserted into shared storage
  -> pill window receives correction-learned event
```

### Future Output Flow

```
STT output
  -> correction store applies known wrong -> corrected mappings seen at least twice
  -> polish receives corrected text
  -> delivered text improves over time
```

## Data Contract

```typescript
interface CorrectionLearningFile {
  version: 1;
  updated_at_ms: number;
  corrections: CorrectionMapping[];
}

interface CorrectionMapping {
  wrong: string;
  corrected: string;
  frequency: number;
  first_seen_at_ms: number;
  last_seen_at_ms: number;
  source: "post_delivery_edit";
}

interface CorrectionLearnedEvent {
  wrong: string;
  corrected: string;
  frequency: number;
}

interface AppSettings {
  correction_memory_enabled: boolean;
}
```

## Acceptance Criteria

1. When correction memory is enabled, after text delivery the backend starts a
   short-lived edit observer.
2. If the observer can read a stable user edit, it stores a `wrong -> corrected`
   mapping.
3. The storage path is shared across app variants and survives restarts.
4. The system never stores full target documents as learned data.
5. Future transcription text applies learned mappings before polish/delivery
   only after the same mapping has been observed at least twice.
6. The pill window shows a subtle tooltip when a new mapping is learned.
7. The feature is best-effort: unsupported platforms or inaccessible text
   fields fail silently with debug logs.
8. The user can disable correction memory in Settings.
9. The user can clear all locally remembered correction pairs in Settings.
10. Unit tests cover diff extraction, storage upsert, clear, and correction
    application thresholding.

## BDD Scenarios

### Learn a correction pair

Given AriaType delivered `这个分析错误需要修复`
When the focused editor later contains `这个分词错误需要修复`
Then the shared correction file stores `分析 -> 分词`
And the pill window receives a correction learned event.

### Ignore non-correction edits

Given AriaType delivered `hello`
When the focused editor later contains `hello world`
Then no correction mapping is stored.

### Apply learned corrections

Given the shared correction file contains `分析 -> 分词`
And the mapping frequency is at least 2
When future STT output contains `这个分析错误需要修复`
Then AriaType sends `这个分词错误需要修复` into polish and delivery.

### Do not apply first observation

Given the shared correction file contains `分析 -> 分词`
And the mapping frequency is 1
When future STT output contains `这个分析错误需要修复`
Then AriaType keeps the text unchanged.

### Clear local memory

Given correction memory has stored mappings
When the user clicks Clear Correction Memory in Settings
Then the shared correction file is removed.

## Verification

1. Unit test diff extraction, including CJK term correction and ignored
   insertion-only edits.
2. Unit test correction storage upsert, clear, and deterministic application
   after the minimum frequency threshold.
3. Run Rust test and clippy for backend integration.
4. Run desktop TypeScript, i18n, unit test, and build checks for pill UI wiring.

## Privacy Notes

Correction Learning stores only bounded correction pairs. The observer discards
full focused text snapshots after extracting a mapping.
