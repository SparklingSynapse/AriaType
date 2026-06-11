# CapsWriter-Offline Polish Latency Analysis

> Last verified: 2026-06-08
> Upstream snapshot: HaujetZhao/CapsWriter-Offline `master`
> commit `4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e`

## Purpose

This document explains why CapsWriter-Offline can feel fast when users compare
its "polish" behavior with AriaType local polish. The goal is not to copy its
implementation directly. The goal is to separate latency sources, identify the
product and architecture choices that create the fast user experience, and turn
those findings into concrete AriaType design guidance.

## Executive Summary

CapsWriter-Offline feels fast mostly because LLM polish is not always on the
critical path.

Its fast path is:

1. Run local STT.
2. Apply deterministic post-processing such as hotwords and regular-expression
   replacement.
3. Output text immediately when no enabled LLM role matches.
4. When LLM processing is enabled, call an external, already-running service
   such as Ollama, LM Studio, or an OpenAI-compatible cloud endpoint.
5. Stream LLM chunks and optionally type them into the focused app as soon as
   each chunk arrives.

AriaType local polish previously had a different performance shape:

1. It invokes in-process llama.cpp work from the desktop backend.
2. The shared blocking path validates the model file, initializes llama.cpp,
   loads the GGUF, creates a context, performs prompt prefill, then generates.
3. It waits for the final polish result before text injection.
4. Recent changes added wall-clock timeout, no-thinking hints, and larger output
   budget, which improve reliability but can increase memory pressure and make
   slow local models more visible.

The practical takeaway: to approach CapsWriter's perceived latency, AriaType
should treat LLM polish as a tiered enhancement, not as the default correction
path. Deterministic correction should handle the common case; LLM polish should
be explicit, streamed, bounded, and preferably backed by a resident model
runtime or a fast cloud model.

## Source Map

CapsWriter-Offline sources used for this analysis:

| Area | Source |
|------|--------|
| README performance and feature overview | [README](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/README.md) |
| LLM role user documentation | [docs/角色功能如何使用.md](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/docs/%E8%A7%92%E8%89%B2%E5%8A%9F%E8%83%BD%E5%A6%82%E4%BD%95%E4%BD%BF%E7%94%A8.md) |
| Default polish role | [LLM/default.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/LLM/default.py) |
| Role matching | [llm_role_detector.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_role_detector.py) |
| LLM orchestration | [llm_handler.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_handler.py) |
| OpenAI/Ollama streaming | [llm_processor.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_processor.py) |
| Streaming typing output | [llm_output_typing.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_output_typing.py) |
| Client reuse | [llm_client_pool.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_client_pool.py) |
| Context trimming | [llm_context.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_context.py) |
| Message construction | [llm_message_builder.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_message_builder.py) |
| LLM defaults and timeouts | [llm_constants.py](https://github.com/HaujetZhao/CapsWriter-Offline/blob/4e4e16bbabdb5b4ebd522cc9f58528b48c87e08e/core/client/llm/llm_constants.py) |

AriaType sources used for comparison:

| Area | Source |
|------|--------|
| Local polish invocation and timeout | [`commands/audio/polish.rs`](../../../apps/desktop/src-tauri/src/commands/audio/polish.rs) |
| OpenAI-compatible localhost polish path | [`polish_engine/local_http.rs`](../../../apps/desktop/src-tauri/src/polish_engine/local_http.rs) |
| Local polish runtime readiness | [`polish_engine/local_runtime.rs`](../../../apps/desktop/src-tauri/src/polish_engine/local_runtime.rs) |
| Polish manager instance cache | [`polish_engine/unified_manager.rs`](../../../apps/desktop/src-tauri/src/polish_engine/unified_manager.rs) |
| Qwen local polish engine wrapper | [`polish_engine/qwen/engine.rs`](../../../apps/desktop/src-tauri/src/polish_engine/qwen/engine.rs) |

## CapsWriter Pipeline

The effective runtime behavior can be represented as:

```text
recording
  -> local STT
  -> hotword / regex / punctuation post-processing
  -> role detection
       -> no enabled role: output original processed transcript
       -> enabled role:
            -> build compact messages
            -> call resident external LLM service or cloud endpoint
            -> stream response chunks
            -> typing mode: write chunks as they arrive
            -> paste mode: paste full response after stream completes
```

The important detail is the branch after role detection. If no enabled role
matches, CapsWriter does not wait for an LLM at all.

## Latency Sources in CapsWriter

### 1. LLM Polish Is Disabled by Default for the Default Role

CapsWriter has a default role for transcript cleanup and hotword-aware polish,
but the current default role file sets that role to disabled. The role detector
only uses the default role when it is enabled; otherwise it returns no role and
the handler writes the text without LLM processing.

Evidence:

- `LLM/default.py` has `enabled = False`.
- `llm_role_detector.py` skips disabled roles and only falls back to the default
  role when it is enabled.
- `llm_handler.py` directly outputs text when role detection returns no role.

This means the common path can be fast even when the product contains an LLM
polish feature.

Implication for AriaType:

- Do not make "LLM polish" synonymous with "basic transcript correctness".
- The basic path should be STT plus correction learning, hotwords, punctuation,
  and formatting rules.
- LLM polish should be a selectable enhancement or a model-backed mode with a
  clear latency tradeoff.

### 2. Deterministic Hotword and Regex Correction Handles the Common Case

CapsWriter advertises hotword replacement and regex replacement as core
features. These are deterministic or near-deterministic post-processing steps,
not generative LLM calls.

Why this is fast:

- Hotword/regex correction has predictable runtime.
- It does not need model loading.
- It does not need autoregressive decoding.
- It can run before or instead of LLM polish.

Why this matters:

- Many user complaints described as "polish quality" are actually lexical
  correction problems: product names, project names, app names, people's names,
  symbols, file extensions, and repeated STT homophones.
- LLMs are expensive for this class of problem because they re-generate the
  whole text to fix a small number of tokens.

AriaType already has correction-learning work. That should become the first
latency-sensitive correction layer, with LLM polish reserved for transformations
that rules cannot safely perform.

### 3. External Resident Runtime Avoids Per-Request GGUF Loading

CapsWriter's LLM processor does not load GGUF files directly. It obtains a
client from a cached client pool and calls:

- Ollama through its native client, or
- LM Studio through an OpenAI-compatible endpoint, or
- a cloud OpenAI-compatible provider such as DeepSeek.

The model runtime is outside the CapsWriter process. For local models, the
expensive work is owned by Ollama or LM Studio, which can keep the model warm.
For cloud models, model loading is hidden by the provider.

Before the resident-runtime migration, AriaType local polish differed in a
critical way:

- `UnifiedPolishManager` caches a `PolishEngineInstance`, but the shared local
  polish function still calls llama.cpp `load_from_file` and creates a new
  context inside the blocking request path.
- This makes "preload" cheaper than full inference only at the wrapper level;
  it does not prove the GGUF weights and KV/runtime state are resident across
  polish calls.

The current implementation has moved local GGUF polish to an OpenAI-compatible
localhost runtime, so this is now the historical latency problem the redesign is
meant to avoid.

Implication for AriaType:

- A truly fast local polish mode needs a resident model runtime.
- Options:
  - Maintain a real in-process `LlamaModel` cache with safe ownership and
    lifecycle control.
  - Use a local server process such as Ollama, LM Studio, llama-server, vLLM, or
    another OpenAI-compatible resident runtime.
  - Keep current in-process GGUF as an "accurate/offline fallback" rather than
    the default fast path.

### 4. Streaming Reduces Perceived Latency

CapsWriter uses streaming for both Ollama and OpenAI-compatible providers. The
processor accumulates the full response, but it also calls a callback whenever a
content chunk arrives.

The typing output mode uses that callback to write chunks into the focused app
as they arrive. Paste mode waits for the full response, but typing mode optimizes
first-visible-output latency.

This changes the product metric:

- Non-streaming polish optimizes "final result latency".
- Streaming typing optimizes "time to first visible text".

The tradeoff is important:

- Streaming typing is fast to perceive.
- It is harder to correct already-typed output if the model changes direction.
- It can produce partial text if the user interrupts.
- It is more suitable for assistant/continuation roles than strict transcript
  replacement.

Implication for AriaType:

- Do not stream directly into the user's target input by default for polish,
  because AriaType's polish is expected to produce a final corrected transcript.
- A safer version is:
  - stream into pill tooltip or a transient preview;
  - paste the final text only after completion;
  - optionally provide an advanced "stream typing" mode for users who explicitly
    prefer speed over atomic correctness.

### 5. Thinking Is Disabled by Default

CapsWriter role configuration defaults `enable_thinking` to false. The OpenAI-
compatible path sends an extra body to disable thinking for providers that
support it. The Ollama path also passes the role's thinking flag.

This is important because polish should usually be a low-reasoning task:

- The output should preserve user intent.
- The model should not answer questions.
- The model should not plan, execute, or reinterpret commands.
- The model should not spend tokens on hidden or visible reasoning.

Implication for AriaType:

- Keep no-thinking as the default for local and cloud polish.
- Maintain model-specific controls because "disable thinking" is not portable.
- Keep `<think>...</think>` stripping as a defensive output sanitizer.
- Avoid thinking-only local models for fast polish.

### 6. Context Is Short and Actively Trimmed

CapsWriter's role config defaults `max_context_length` to 4096 tokens. Its
context manager trims old history when the estimated history size exceeds an
80% threshold, leaving room for output.

The message builder also keeps context targeted:

- system prompt;
- optional history;
- matched hotwords;
- optional selected text;
- current user input.

It does not blindly attach a large window snapshot to every request.

Implication for AriaType:

- Keep polish prompt context small by default.
- Include only high-confidence correction candidates, not broad visible text.
- Put window context behind a strict budget and use it as lexical hints only.
- Use top-K correction/hotword retrieval instead of full dictionaries.

### 7. Timeout Is Aggressive

CapsWriter's LLM constants set a short default timeout for local and cloud
providers. This makes failures quick and preserves typing flow. It also means
CapsWriter can feel fast because slow LLM calls fail or degrade early.

Implication for AriaType:

- AriaType should expose different latency contracts for different modes:
  - correction-only: sub-100ms target after STT;
  - cloud-fast polish: low single-digit seconds;
  - local-fast resident model: low single-digit seconds after warmup;
  - local-accurate GGUF fallback: bounded, slower, and visibly labelled.
- Fallback must be explicit: use original STT and show a short tooltip reason.

## Current AriaType Local Polish Shape

The current local polish flow is:

```text
maybe_polish_transcription_text
  -> deterministic post-STT correction pipeline
  -> run_local_polish
      -> compute local timeout
      -> create PolishRequest
      -> attach optional preview callback
      -> tokio timeout around manager.polish
          -> manager gets/caches engine wrapper
          -> verify the GGUF file is complete
          -> check or start OpenAI-compatible localhost runtime
          -> POST /v1/chat/completions
          -> stream chunks into hidden pill processing state
          -> optionally stream chunks into the target app in advanced mode
      -> accept result or fallback original STT
```

Recent improvements:

- Local polish has a wall-clock timeout.
- Local GGUF polish now uses an OpenAI-compatible localhost runtime, so the
  desktop request path no longer performs direct llama.cpp generation.
- Startup warmup preloads the configured polish model independently from STT
  residency when the model file and runtime are available.
- Local models use no-thinking hints by default where supported.
- Outputs are sanitized for `<think>` blocks.
- Max generated output budget was raised to avoid truncated thinking/output
  failures.
- Streaming polish keeps the pill in a hidden processing state.
- Direct streaming typing exists as an explicit advanced setting and is disabled
  by default.

Remaining latency issue:

- AriaType can reuse an already-running localhost runtime, spawn a bundled
  `llama-server` resource, use a PATH-installed `llama-server`, or spawn a
  configured command. Build scripts now add existing `llama-server` sidecars to
  Tauri resources through a generated config, but the repository does not yet
  include the actual bundled sidecar binary.
- First-use latency still depends on the chosen runtime's model loading and
  readiness behavior.
- The safe default UI waits for final polish text before injection, so perceived
  latency remains close to final latency unless advanced direct streaming typing
  is enabled.

## Side-by-Side Comparison

| Dimension | CapsWriter-Offline | AriaType Current |
|-----------|--------------------|------------------|
| Default LLM polish | Default role disabled | User can enable polish as processing step |
| Basic correction | Hotwords and regex are first-class | Correction learning exists, still maturing |
| Local LLM runtime | External resident service such as Ollama/LM Studio | OpenAI-compatible localhost runtime |
| Model warmup | Owned by external service | Existing runtime reused or configured command spawned |
| Output behavior | Can stream chunks into target app | Tooltip streaming by default; direct typing is advanced and off |
| Context budget | Default 4096 with trimming | Larger local context after recent change |
| Thinking | Disabled by default | Disabled by default plus stripping |
| Timeout | Very aggressive provider timeout | 10-30s local timeout |
| Product tradeoff | Optimizes perceived speed | Optimizes atomic final transcript |

## Recommended AriaType Architecture

### 1. Split Correction From Polish

Introduce or formalize a post-STT pipeline with distinct stages:

```text
raw transcript
  -> normalization
  -> correction learning mappings
  -> hotword / glossary replacement
  -> punctuation and spacing rules
  -> optional LLM polish
  -> final injection
```

The first four stages should be deterministic and fast. LLM polish should not be
required for basic spelling correctness.

### 2. Add Polish Execution Modes

Recommended modes:

| Mode | Purpose | Latency target | Fallback |
|------|---------|----------------|----------|
| Off | Raw STT plus deterministic corrections | Fastest | N/A |
| Correction Only | Fix known words, punctuation, spacing | <100ms after STT | Original STT |
| Cloud Fast | High-quality quick rewrite via fast cloud model | 1-3s typical | Deterministic output |
| Local Resident | Ollama/LM Studio/llama-server style runtime | 1-5s after warmup | Deterministic output |
| Local Accurate | Current GGUF path for offline quality | 10-30s bounded | Deterministic output |

This makes latency a product contract instead of an accidental consequence of
model selection.

### 3. Make Resident Local Runtime the Fast Local Path

For fast local polish, prefer a resident runtime:

- Ollama for broad local model compatibility;
- LM Studio for user-managed desktop local models;
- llama-server for direct llama.cpp control;
- a dedicated sidecar if AriaType needs fully managed offline behavior.

The key requirement is that model load is not part of every polish request.

If AriaType keeps an in-process model cache, it must cache actual loaded model
state, not only wrapper objects. The design must address:

- ownership and thread safety;
- memory pressure and unload policy;
- model switch lifecycle;
- cancellation behavior;
- context reuse vs per-request context creation;
- crash isolation.

### 4. Stream to Preview Before Streaming to Input

Add a streaming callback to the polish engine contract, but keep final injection
atomic by default.

Recommended default UX:

```text
LLM chunk arrives
  -> keep pill tooltip in generic processing state
  -> keep target input unchanged
LLM completes
  -> inject final text
```

Optional advanced UX:

```text
LLM chunk arrives
  -> type chunk directly into target input
ESC / cancel
  -> stop stream and keep partial text
```

The default should preserve AriaType's current guarantee that the target field
receives a complete final transcript.

### 5. Keep Prompt Budgets Small by Default

Recommended budget policy:

- Fast polish: cap output tightly for transcript-length-preserving tasks.
- Accurate polish: allow larger output but keep timeout visible.
- Thinking-only models: mark as not recommended for fast polish.
- Context: include only top-K correction and hotword candidates.
- Window context: include only compact lexical hints and only when confidence is
  high.

The recent 20x local output budget is useful as a safety valve against
truncation, but it should not be the default fast-path behavior for all local
models.

### 6. Instrument User-Visible Latency

Add or standardize these trace fields:

| Field | Meaning |
|-------|---------|
| `stt_ms` | STT inference duration |
| `postprocess_ms` | deterministic correction duration |
| `polish_queue_ms` | time before polish starts |
| `model_load_ms` | local model load duration |
| `context_create_ms` | local context creation duration |
| `prefill_ms` | prompt prefill duration |
| `time_to_first_token_ms` | first streamed chunk latency |
| `generation_ms` | decode duration |
| `injection_ms` | text injection duration |
| `fallback_reason` | reason original or deterministic text was used |

Without these fields, "polish is slow" is ambiguous.

## Migration Plan

### Phase 1: Make the Fast Path Explicit

- Ensure deterministic correction is a named pipeline stage.
- Add logs for correction mapping hits and hotword candidates.
- Keep LLM polish optional and clearly labelled.
- Preserve current local polish fallback behavior.

Current implementation status:

- Recording and retry flows share a named post-STT processing stage before LLM
  polish.
- The stage applies conservative text normalization before correction memory and
  after glossary mappings: whitespace is collapsed, spaces before punctuation
  are removed, common Latin separator spacing is repaired, and CJK punctuation
  spacing is cleaned without invoking an LLM.
- The stage applies correction-learning mappings when enabled and keeps output
  deterministic when no polish template is selected.
- `post_stt_pipeline_completed` logs `postprocess_ms`,
  `normalization_applied`, `corrections_applied`, `glossary_applied`,
  glossary entry count, input/output length, and fallback reason.
- Final recording and retry completion logs include `postprocess_ms`,
  `normalization_applied`, `corrections_applied`, `glossary_applied`,
  `polish_ms`, `polish_wall_ms`, `polish_queue_ms`, `model_load_ms`,
  `context_create_ms`, `prefill_ms`, `inference_ms`,
  `time_to_first_token_ms`, `generation_ms`, and `fallback_reason`.
- Final text delivery logs `text_injection_completed` with `injection_ms`,
  context, entry id when applicable, task id, and text length.
- `stt_engine_user_glossary` is now also a deterministic post-STT stage:
  comma/newline separated terms keep STT hint behavior and canonical ASCII
  casing, while explicit `wrong -> corrected` / `wrong => corrected` pairs are
  applied after STT and before LLM polish.
- OpenAI-compatible local responses preserve llama.cpp-style
  `timings.prompt_ms` / `timings.predicted_ms` as `prefill_ms` /
  `inference_ms`, and AriaType-managed sidecars can expose
  `ariatype_timings.model_load_ms`, `context_create_ms`, `prefill_ms`, and
  `inference_ms`.
- `model_load_ms` and `context_create_ms` are populated only when the runtime
  exposes them; stock OpenAI providers usually omit these fields.

### Phase 2: Add Streaming Contract

- Extend polish engine results to support optional chunk callbacks.
- Stream cloud polish chunks into a hidden pill processing state first.
- Add tests that verify final injection remains atomic unless a streaming typing
  mode is explicitly enabled.

Current implementation status:

- `PolishRequest` supports an optional preview callback, and `PolishResult`
  records `time_to_first_token_ms` plus `generation_ms` when the provider uses a
  streaming response.
- OpenAI-compatible cloud polish and OpenAI-compatible local polish can request
  `stream: true`, parse SSE `data:` chunks, and drive a generic pill processing
  state without exposing raw streamed text.
- The preview channel is UI-only. Recording and retry flows still wait for the
  final polish result before text insertion, so target input injection remains
  atomic by default.
- Any preview accumulation still filters incomplete `<think>` blocks so local
  thinking output is not surfaced in user-visible states.
- Direct streaming typing into the target input is implemented as an explicit
  advanced setting, disabled by default, and limited to the recording flow.
  Retry keeps atomic final-result insertion.
- Anthropic polish remains final-response only.

### Phase 3: Add Local Resident Provider

- Add an OpenAI-compatible local provider preset for:
  - Ollama;
  - LM Studio;
  - llama-server.
- Reuse the cloud polish HTTP client shape where possible.
- Add a health check and a "model warm" status.
- Keep in-process GGUF as offline fallback.

Current implementation status:

- Local GGUF polish requests use an OpenAI-compatible localhost endpoint.
- Startup warmup and first-use polish now check local runtime readiness.
- Existing local runtimes are accepted when the configured base URL is already
  listening.
- Private AI > Polish exposes local runtime presets for llama-server, LM
  Studio, Ollama, and custom OpenAI-compatible endpoints.
- Users can run a local runtime readiness check from the UI before relying on
  local polish. When the selected polish model is already downloaded, the check
  uses the same `load_model -> ensure_ready` path as startup warmup and first
  use, so configured, PATH-installed, or bundled `llama-server` processes can be
  started and verified. Without a downloaded selected model, it falls back to a
  lightweight `/v1/models` endpoint health check.
- A managed sidecar can be started via saved runtime command configuration.
  Build and release scripts can bundle `llama-server`; when a packaged runtime
  is absent, AriaType can still use a `llama-server` on `PATH` or a custom
  OpenAI-compatible command.
- For the `llama-server` preset, an empty start command now auto-detects a
  bundled `llama-server` resource, then `llama-server` from `PATH`, before
  reporting the runtime unavailable.
- macOS and Windows build scripts generate `tauri.runtime.generated.conf.json`
  before Tauri packaging, preserving existing resources and adding any present
  `llama-server` sidecar resources.
- Build-time sidecar preparation can copy a release-provided `llama-server`
  artifact into the recognized Tauri resource location, validate its optional
  SHA-256 checksum, and fail fast when
  `ARIATYPE_REQUIRE_LOCAL_POLISH_RUNTIME=1` is set.
- macOS sidecar preparation supports separate arm64 and x64 binaries, and
  runtime discovery now prefers the current architecture before trying the other
  architecture. This keeps universal macOS packages from launching the wrong
  `llama-server` binary on Intel machines.
- The release workflow pins a llama.cpp release tag, downloads the official
  macOS arm64, macOS x64, and Windows CPU x64 assets, extracts `llama-server`,
  and enables the required-runtime gate during packaging.
- The pinned `b9568` release currently uses
  `llama-b9568-bin-macos-arm64.tar.gz`,
  `llama-b9568-bin-macos-x64.tar.gz`, and
  `llama-b9568-bin-win-cpu-x64.zip`; tests cover both tar and the official
  Windows zip archive shape.
- Release asset preparation copies the runtime executable and sibling dynamic
  library dependencies from the same archive directory: `.dylib` on macOS,
  `.dll` on Windows, and `.so*` on Linux. This is required because the official
  macOS `llama-server` fails to launch if only the executable is bundled.
- Release packaging now verifies that the resulting macOS app bundle or Windows
  bundle still contains the expected sidecar resources, and smoke-executes the
  current-architecture `llama-server` with `--help` using a 30s timeout, before uploading
  installers.
- The runtime verifier has an optional real-model server smoke mode that starts
  the packaged `llama-server` and waits for `/v1/models` when a GGUF path is
  supplied.
- `get_polish_model_status` distinguishes downloaded model files from runtime
  readiness with `is_downloaded` and `runtime_ready`.
- The Private AI > Polish UI displays local runtime readiness separately from
  model-file download status, so a downloaded model is not presented as fully
  ready when the local runtime is unavailable.

### Phase 4: Improve Model Selection

- Mark models as:
  - fast transcript-preserving;
  - accurate rewrite;
  - forced-thinking;
  - high-memory;
  - not recommended for fast polish.
- Warn users when a selected local model is likely to exceed the fast-path
  latency target.

Current implementation status:

- Local polish models now expose a `latency_profile` with a latency class,
  profile code, recommended templates, and caution templates.
- The model settings UI shows the latency class and template fit next to each
  local polish model, while preserving the existing device compatibility
  warnings.
- GLM-4.7-Flash-REAP-23B-A3B is classified as a heavy long-context model and
  remains covered by the high-memory compatibility warning path.
- Latency profiles are static guidance, not runtime benchmarks. They should be
  recalibrated if bundled quantization, runtime backend, or default templates
  change.

### Phase 5: Tune UX

- Show short tooltip on slow/fallback behavior.
- Distinguish "polishing", "hidden processing state", and "using original transcript".
- Do not show scary errors for expected timeout fallback.
- Provide a per-template latency expectation.

Current implementation status:

- Local polish timeout fallback emits `Local polish timed out. Using original
  transcription.` through the pill tooltip path.
- The selected local polish model row distinguishes fully ready local polish
  from "model file downloaded, runtime not ready" and points users to the local
  runtime check before they rely on local polish.
- Streaming polish keeps the tooltip in a generic processing state while final
  input insertion remains atomic.
- Direct streaming typing is available behind an advanced Performance setting
  and skips the final duplicate insertion when chunks have already been typed.
- Model cards and the selected-model row show per-template latency expectation
  using the same restrained settings UI pattern as device compatibility
  warnings.

## Testing Requirements

Minimum tests before shipping a CapsWriter-inspired fast polish redesign:

1. No-LLM path test: when LLM polish is off, transcript output does not call any
   polish provider.
2. Correction-only test: known wrong-to-right mappings apply without LLM.
3. Role/template test: template selection does not accidentally enable slow
   local polish.
4. Streaming polish state test: OpenAI-compatible chunks keep the pill in the
   processing state while final injection waits for the full result.
5. Streaming typing test: direct chunk typing only occurs when explicitly
   enabled.
6. Timeout test: slow provider falls back to deterministic text and emits a
   user-facing tooltip.
7. Thinking sanitizer test: `<think>` blocks are removed or rejected.
8. Model warmup test: resident provider reports readiness before being shown as
   fast/available.
9. Metrics test: every successful and fallback path records timing fields.

Current verification evidence:

| Requirement | Current evidence |
|-------------|------------------|
| No-LLM path | `commands::audio::tests::no_template_polish_path_does_not_call_provider` |
| Correction-only | `commands::audio::postprocess::tests::reports_correction_only_output_and_applied_count`, `applies_glossary_after_correction_memory` |
| Normalization/punctuation | `commands::audio::postprocess::tests::normalizes_spacing_and_punctuation_without_llm`, `normalizes_before_correction_failure_fallback`, `preserves_decimal_versions_and_time_like_colons`, `removes_spaces_around_cjk_punctuation` |
| Role/template guard | `commands::audio::tests::template_selection_without_model_does_not_pick_local_model_implicitly` |
| Streaming callback plumbing | `cloud_polish_mock_test::test_openai_polish_streams_preview_chunks`, `polish_engine::local_http::tests::streams_openai_chunks_to_preview_callback` |
| Streaming typing | `commands::audio::shared::tests::direct_stream_typing_requires_explicit_non_final_update`, `direct_stream_delta_returns_only_new_visible_text`, `services::transcription_finalize::tests::finalize_successful_transcription_skips_delivery_when_text_was_already_inserted` |
| Timeout fallback | `commands::audio::polish::tests::local_polish_timeout_*`, `commands::audio::shared::tests::local_polish_timeout_tooltip_is_specific`, `cloud_polish_mock_test::test_short_cloud_polish_times_out_after_core_prompt_timeout` |
| Thinking sanitizer | `commands::audio::shared::tests::polish_preview_tooltip_hides_incomplete_thinking`, `commands::audio::shared::tests::direct_stream_delta_hides_incomplete_thinking`, `polish_engine::local_http::tests::strips_complete_think_block` |
| Model warmup/readiness | `polish_engine::local_runtime::tests::treats_existing_listener_as_ready_without_spawn_command`, `reports_missing_runtime_when_no_listener_or_command_exists`, `finds_bundled_llama_server_under_resource_subdir`, `finds_llama_server_on_path_for_default_provider` |
| Runtime readiness UI | `components/Home/__tests__/PolishSection.test.tsx`, `commands::settings::__test__::local_runtime_check_*` |
| Sidecar packaging config | `scripts/prepare-tauri-runtime-resources.test.mjs`, `scripts/build-all-platforms.test.mjs` |
| Sidecar architecture selection | `polish_engine::local_runtime::tests::macos_bundled_runtime_subdirs_prefer_current_architecture`, `scripts/prepare-tauri-runtime-resources.test.mjs` |
| Release sidecar preparation | `scripts/prepare-llama-server-release-assets.test.mjs`, `scripts/release-workflow.test.mjs`, dependency-copy tests for `.dylib` and `.dll` assets |
| Packaged sidecar verification | `scripts/verify-tauri-runtime-resources.test.mjs`, `scripts/release-workflow.test.mjs`, `smokeRuntimeResources`, `smokeRuntimeServer` |
| Metrics | `commands::audio::tests::streaming_finalization_honors_cloud_polish_settings`, `polish_engine::local_http` response timing tests |

Remaining higher-level evidence gap:

- A local integration smoke has proven the release-preparation path with the
  official `b9568` macOS arm64 and x64 assets plus a real downloaded
  `Qwen3.5-0.8B-Q5_K_M.gguf` model: the prepared arm64 `llama-server`
  executable answered `--help`, started with the real GGUF model, and served
  `/v1/models`.
- A local unsigned Tauri macOS arm64 package build has also proven the app
  bundle resource path: `AriaType Inhouse.app/Contents/Resources/bin/apple-silicon/llama-server`
  answered `--help`, started with the real GGUF model, and served `/v1/models`.
- The unsigned macOS arm64 DMG has been mounted locally and verified from the
  mounted app bundle resource directory:
  `/private/tmp/ariatype-dmg-mount/AriaType Inhouse.app/Contents/Resources/bin/apple-silicon/llama-server`
  answered `--help`, started with the real GGUF model, and served `/v1/models`.
- Packaging/E2E has not yet proven the same flow from a final notarized app.
  Current implementation supports existing runtimes, explicit
  commands, bundled-resource discovery, PATH auto-detected `llama-server`,
  build-time sidecar artifact injection, dynamic-library dependency packaging,
  a required-artifact gate, release-workflow sidecar download/extraction,
  packaged-resource verification, executable smoke verification, and an
  optional `/v1/models` server smoke when a real model path is provided. The
  repository still does not commit a real bundled sidecar binary; release builds
  fetch it from the pinned upstream llama.cpp release.

## Risks

### Streaming Directly Into the Target Input

This improves perceived speed but weakens atomic correctness. It can leave
partial output if interrupted and makes later correction difficult. Use it only
as an explicit mode.

### Resident Local Runtime

A resident model service improves latency but introduces lifecycle complexity:

- install and startup;
- health checks;
- port conflicts;
- model availability;
- memory pressure;
- crash isolation;
- cross-platform packaging.

### Overusing LLM for Lexical Corrections

LLMs can fix errors, but they can also rewrite intent. Known word mappings,
glossaries, and hotwords should be handled before LLM polish.

### Large Output Budget

A large budget prevents truncation but can make runaway generation more
expensive. Pair it with:

- no-thinking controls;
- deadline checks;
- stop sequences where possible;
- transcript-preserving prompts;
- preview/fallback UX.

## Decision Guidance

Use this rule of thumb:

- If the problem is "wrong word to right word", use correction learning,
  hotwords, glossary, or regex.
- If the problem is "punctuation, spacing, sentence boundaries", use
  deterministic formatting first.
- If the problem is "make this more readable while preserving intent", use fast
  cloud or resident local LLM.
- If the problem is "high-quality rewrite", use accurate polish and accept
  higher latency.

The CapsWriter lesson is not "LLM polish can always be instant." The lesson is
"do not put expensive generative work on the default path unless the user asked
for that tradeoff."
