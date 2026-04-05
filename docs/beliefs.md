# AriaType Core Beliefs and Non-Negotiable Constraints

This document defines the mechanical rules that keep the AriaType codebase legible and consistent for future agent runs. These beliefs and constraints are non-negotiable. They override implementation convenience, personal preference, and undocumented convention.

---

## Section 1: Core Beliefs

**1. Privacy is non-negotiable**

Voice data never leaves the device by default. Cloud features are opt-in. User trust is the product's foundation.

**2. Accuracy over speed**

STT accuracy > STT stability > user experience > speed. We never accept latency gains that reduce accuracy or stability.

**3. Agent-first by default**

The repository is optimized for agent legibility. If an agent cannot access it in-context, it effectively does not exist. All knowledge must be repository-local and versioned.

**4. Progressive disclosure**

Documents should be maps, not encyclopedias. Start small, point to deeper sources. A 100-line table of contents beats a 1000-line manual.

**5. Spec-first development**

Never code from vague intuition. Identify behavior, constraints, and acceptance criteria first. Features are driven by specs at `docs/feat/[name]/[version]/prd/erd.md`.

**6. Enforce invariants mechanically**

Conventions that are not enforced drift. If it matters, encode it in a test, linter, or CI check. If it cannot be encoded, it is a suggestion, not a rule.

**7. Fail fast with evidence**

Every failure must have a traceable root cause. No silent degradation. No swallowing errors. No "maybe it works."

**8. Replace, don't deprecate**

When a new implementation replaces an old one, remove the old one entirely. No backward-compatible shims, dual config formats, or migration paths.

**9. Boring technology wins**

Dependencies and abstractions should be fully internalizable and reason-about-able in-repo. Prefer stable, composable, well-understood tools.

**10. Technical debt is a high-interest loan**

Pay it down continuously in small increments. Letting it compound is unacceptable. Run doc-gardening and code-gardening on regular cadence.

---

## Section 2: Non-Negotiable Constraints

### Product Priority Order

```
STT accuracy > STT stability > user experience > speed
```

- Do NOT accept latency gains that reduce accuracy or stability.
- Do NOT accept latency gains that degrade user experience.
- Speed is optimized ONLY after accuracy, stability, and UX are protected.
- When in doubt: prefer reliability, validation, clearer state, safer fallback.

### Volcengine Interface Selection

- **Required**: `wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream`
- **Reason**: Bidirectional interfaces (`bigmodel_async`, `bigmodel`) have lower accuracy.
- **Exception**: Only with explicit user request + accuracy impact measured and documented + fallback to `bigmodel_nostream` provided.

### Language Codes (IETF BCP 47)

All language identifiers MUST use IETF format: `language-REGION` (both parts required).

| Correct | Incorrect |
|---------|-----------|
| `en-US` | `en` |
| `zh-CN` | `zh` |
| `ja-JP` | `ja` |
| `ko-KR` | `ko` |
| `zh-TW` | `zh` (for Traditional Chinese Taiwan) |

Reference: RFC 5646.

### Harness Engineering Rules

- Agent owns testing and verification. Do not wait for the user to ask for tests.
- Never invent APIs, files, commands, test results, or behavior not grounded in repo or user instructions.
- Never claim done without running verification. Never present mocks or stubs as finished.
- All code, identifiers, comments, test names, and commit messages must be in English.
- Do not use git to modify git history without explicit user request.
- Suppressing type errors with `as any`, `@ts-ignore`, or `@ts-expect-error` is forbidden.
- Empty catch blocks are forbidden.
- Deleting failing tests to "pass" is forbidden.

---

## Section 3: Context Management Principles

**1. Context is a scarce resource**

A giant instruction file crowds out the task, code, and relevant docs.

**2. Too much guidance becomes non-guidance**

When everything is "important," nothing is.

**3. Documentation rots instantly**

A monolithic manual becomes a graveyard of stale rules. Enforce freshness mechanically.

**4. Verify, don't trust**

A single blob does not lend itself to mechanical checks (coverage, freshness, ownership, cross-links), so drift is inevitable.

**5. Dependency graph beats file tree**

When exploring code, trace direct dependencies, implementations, callers, and utilities. This reduces relevant files from 65 to 6-8 on average.
