# Quality

Quality index tracking grades across all product domains. Grades are updated per sprint or after significant changes.

## Grade Scale

| Grade | Meaning | Requirements |
|-------|---------|-------------|
| **A** | Fully verified | Automated tests + manual QA pass, no known regressions |
| **B** | Testable | Manual verification works, no known regressions, automated tests partial |
| **C** | Build-verified | Compiles and builds, no automated tests, manual verification not done |
| **D** | Known issues | Has known bugs or regressions, needs attention |

## Per-Domain Quality Grades

| Domain | Layer | Grade | Last Verified | Notes |
|--------|-------|-------|--------------|-------|
| Audio Recording | Backend | B | 2026-04 | Manual testing, integration tests partial |
| Audio Resampling | Backend | B | 2026-04 | Unit tests for sample rate conversion |
| VAD (Voice Activity Detection) | Backend | C | 2026-04 | Build-verified only |
| Local STT (Whisper) | Backend | B | 2026-04 | Integration tests, manual QA |
| Local STT (SenseVoice) | Backend | B | 2026-04 | Integration tests |
| Cloud STT (Volcengine) | Backend | B | 2026-04 | Auth error tests pass |
| Cloud STT (Qwen Omni) | Backend | C | 2026-04 | Missing #[instrument], no contract tests |
| Cloud STT (ElevenLabs) | Backend | C | 2026-04 | Missing #[instrument], no contract tests |
| Cloud STT (Deepgram) | Backend | C | 2026-04 | No contract tests |
| Local Polish (LFM) | Backend | B | 2026-04 | Unit tests for prompt construction |
| Local Polish (Qwen) | Backend | B | 2026-04 | Unit tests |
| Cloud Polish (Anthropic) | Backend | B | 2026-04 | Auth error tests |
| Cloud Polish (OpenAI) | Backend | B | 2026-04 | Auth error tests |
| Text Injection (macOS) | Backend | B | 2026-04 | Layer 0 + Layer 2 tested |
| Text Injection (Windows) | Backend | D | 2026-04 | Not implemented |
| Settings Persistence | Backend | B | 2026-04 | Unit tests |
| History Storage | Backend | C | 2026-04 | Build-verified |
| UI Components | Frontend | C | 2026-04 | TypeScript compiles, no visual regression |
| Settings UI | Frontend | B | 2026-04 | Manual QA, component tests |
| Dashboard UI | Frontend | B | 2026-04 | Manual QA |
| IPC Boundary | Full-stack | B | 2026-04 | Typed boundary, logging |
| Logging Infrastructure | Full-stack | B | 2026-04 | Spec exists, partial coverage |
| i18n | Frontend | A | 2026-04 | Automated check (pnpm check:i18n), 10 locales |
| Website | Frontend | B | 2026-04 | Build passes, 2 locales |
| CI/CD | Infrastructure | B | 2026-04 | Test + release + deploy pipelines |

## Mandatory Coverage Gates

These are hard requirements, not grades:

- **End-to-end**: 100% coverage for the affected user workflow before handoff
- **Unit**: 100% coverage for affected critical core modules
- Enforced per-task, not per-domain

## Improvement Priorities

Sorted by impact:

1. Cloud STT contract tests for Qwen Omni, ElevenLabs, Deepgram (raise from C to B)
2. VAD testing (raise from C to B)
3. Text Injection Windows implementation (raise from D)
4. History storage tests (raise from C to B)
5. Visual regression for UI components (raise from C to B)
