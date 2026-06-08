# Polish Model Device Compatibility Warnings

## Goal

Help users understand whether a local polish model is likely to run smoothly on the current device before they select or download it.

## Scope

- Detect device capability locally without sending hardware information to a remote service.
- Evaluate local polish models using conservative RAM and CPU-thread requirements.
- Return compatibility metadata through the existing polish model list command.
- Show non-blocking warnings in the model settings UI.

## Non-goals

- Do not run a benchmark during settings load.
- Do not block users from selecting or downloading a model.
- Do not infer GPU availability for local polish models in this iteration.
- Do not validate GGUF runtime loadability in this iteration.

## Requirements

- The backend owns compatibility decisions.
- The frontend only renders backend-provided compatibility metadata.
- Compatibility levels:
  - `smooth`: current device meets the recommended requirement.
  - `limited`: model may run, but latency or memory pressure is likely.
  - `unsupported`: current device is below the minimum requirement.
- If total memory cannot be detected, return a conservative warning.
- Warnings must explain the relevant device and model thresholds.

## Acceptance

- `get_polish_models` includes a `compatibility` object for each local polish model.
- Windows devices use native memory detection.
- macOS and Linux use best-effort local memory detection.
- The UI shows warning text for `limited`, `unsupported`, and unknown-memory cases.
- Large local polish models such as GLM-4.7 Flash REAP 23B-A3B warn below the recommended memory threshold.
- The feature is covered by deterministic unit tests for model threshold decisions.
