---
date: 2026-04-16
topic: stt-benchmark
---

# STT Engine Benchmark Requirements

## Problem Frame

AriaType supports 6 STT providers with different latency, accuracy, and cost profiles. Developers and architects need a standardized benchmark to:
- Make informed engine selection decisions
- Quantify performance tradeoffs when changing engines or configurations
- Track performance changes over time (e.g., after API updates, model version changes)

Current gap: No systematic way to compare STT engines under controlled conditions.

## Requirements

**Benchmark Scope**
- R1. Measure **WER (Word Error Rate)** for transcription accuracy across all supported STT engines
- R2. Measure **full cold-start latency** including model loading, connection establishment, and first API roundtrip
- R3. Run benchmark on-demand (not CI-integrated), triggered manually by developers

**Test Dataset**
- R4. Use ~20 samples from open-source STT datasets covering:
  - Chinese and English (bilingual coverage)
  - Short audio (< 10s) and medium audio (10-60s)
  - Clean and noisy environment conditions
- R5. Ground truth transcripts required for WER calculation (from dataset annotations)
- R6. Dataset stored in `tests/fixtures/benchmark/audio/` and `tests/fixtures/benchmark/transcripts/`

**Metrics & Measurement**
- R7. WER calculation: `(insertions + deletions + substitutions) / total_words`
- R8. Cold-start latency breakdown into measurable phases:
  - Engine initialization time
  - Connection/authentication time
  - First chunk processing time
  - Total time from engine create to final transcript
- R9. Run each sample 3 times per engine, report mean ± std deviation

**Output Format**
- R10. Generate Markdown table report with per-engine summary:
  - Engine name, WER mean±std, cold-start latency breakdown
- R11. Output file: `benchmark-results-{YYYY-MM-DD}.md` in project root

## Success Criteria
- A developer can run `cargo bench --bench stt_benchmark` (or similar) and get a comparable report across all engines
- Report includes WER and latency for each engine on the same test samples
- Benchmark completes within reasonable time (target: < 5 min total for all engines)

## Scope Boundaries
- **Not building**: Automated dataset expansion, LLM-assisted annotation, CI integration, web dashboard
- **Not measuring**: Cost/pricing, streaming throughput, error rate on malformed audio
- **Not comparing**: Polish engine performance (separate benchmark if needed)

## Key Decisions
- **Dataset strategy**: Open-source samples (~20) for balance of representativeness and maintainability
- **Metric**: WER for accuracy (standard, comparable with academic benchmarks)
- **Frequency**: On-demand only (low carrying cost, no CI complexity)
- **Output**: Markdown report (human-readable, easy to archive)

## Dependencies / Assumptions
- [Unverified] Need to identify suitable open-source Chinese/English STT datasets with clean transcripts
- Assumes all engines have API keys configured in test environment
- Assumes engines support batch mode (not streaming) for controlled latency measurement

## Outstanding Questions

### Resolve Before Planning
- [Affects R4][Needs research] Which open-source datasets to use?
  - Candidates: AISHELL-1 (Chinese), LibriSpeech (English), Common Voice (multilingual)
  - Need to verify: license compatibility, audio format, transcript availability, download mechanism

### Deferred to Planning
- [Affects R7][Technical] How to compute WER for Chinese vs English text segmentation (word vs character)?
- [Affects R8][Technical] How to instrument engine internals for phase-level timing?
- [Affects R10][Technical] Report format details (single table vs per-language tables)

## Next Steps
→ Resume `/ce:brainstorm` to resolve dataset selection before planning