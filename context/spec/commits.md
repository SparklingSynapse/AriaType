# Commit Message Specification

Conventions and format for AriaType commit messages.

---

## 1. Format

All commits MUST follow Conventional Commits format:

```
type(scope): subject

[optional body]

[optional footer]
```

### Mandatory Rules

1. **Type**: One of the predefined types (Section 2)
2. **Scope**: `desktop`, `website`, or omitted for repo-wide changes (Section 3)
3. **Subject**: Concise description in imperative mood, no period, ≤72 characters
4. **Language**: English only
5. **No trailing period**: Subject line ends without punctuation

### Examples

```
feat(desktop): retry failed transcriptions from history
feat(desktop): cancel recording with ESC key
fix(desktop): prevent long recordings from being truncated
refactor(desktop): faster model loading on startup
chore: bump version to 0.3.0
docs: add v0.3 release notes
ci: add macOS binary pre-signing for smoother install
```

---

## 2. Type Taxonomy

| Type | When | Examples |
|------|------|----------|
| `feat` | New feature or capability | Add retry functionality, new hotkey support |
| `fix` | Bug fix, error correction | Fix audio truncation, correct API response handling |
| `refactor` | Code change without behavior change | Unify engine trait, reorganize modules |
| `chore` | Maintenance, tooling, release | Update version, bump dependencies, clean up |
| `docs` | Documentation only | Update README, fix typo in guide |
| `build` | Build system, dependencies | Add build script, update Cargo.toml |
| `ci` | CI/CD configuration | Update GitHub Actions, add lint step |
| `test` | Adding or modifying tests | Add integration test for pipeline |
| `perf` | Performance improvement | Optimize audio chunking, reduce memory usage |
| `style` | Code style (formatting, whitespace) | Fix indentation, normalize quotes |

---

## 3. Scope Taxonomy

Scope identifies the affected package. Only three scopes are allowed:

| Scope | Package | Notes |
|-------|---------|-------|
| `desktop` | `apps/desktop/*` | Desktop application (Tauri + frontend, including assets, icons, config) |
| `website` | `apps/website/*` | Website and marketing site |
| (none) | Repository-wide | Documentation, CI, release, monorepo-wide changes |

### Scope Rules

- **Use `desktop`** for any change touching `apps/desktop/*` (Rust backend, frontend, assets, config)
- **Use `website`** for any change touching `apps/website/*`
- **Omit scope** for repository-wide changes: documentation (`README.md`, `context/*`), CI/CD, release, tooling, dependencies that span packages
- **Never use invented scopes** (`stt`, `audio`, `ui`, etc.) — these are domains, not packages
- **If change touches both desktop and website** — commit separately, one commit per package

---

## 4. Subject Guidelines

### User-Facing Principle

Subject should describe **what users experience**, not technical implementation:

```
✅ feat(desktop): retry failed transcriptions from history
❌ feat(desktop): add transcription retry functionality with saved audio lookup

✅ fix(desktop): prevent long recordings from being truncated
❌ fix(desktop): fix audio chunk boundary handling in recorder thread

✅ refactor(desktop): faster model loading on startup
❌ refactor(desktop): unify engine trait and lazy-load on background thread
```

Users read commit messages in release notes. They care about:
- "What can I do now that I couldn't before?" (feat)
- "What problem is fixed?" (fix)
- "What behavior changed?" (refactor)

Technical details belong in the body, not the subject.

### Imperative Mood

Use imperative mood ("add" not "added", "fix" not "fixes"):

```
✅ feat(desktop): retry failed transcriptions
❌ feat(desktop): retried failed transcriptions

✅ fix(desktop): prevent recording truncation
❌ fix(desktop): prevented recording truncation
```

### Capitalization

Subject starts with lowercase:

```
✅ feat(desktop): retry failed transcriptions
❌ feat(desktop): Retry failed transcriptions
```

### Length

Subject line ≤72 characters. If longer, move detail to body:

```
✅ feat(desktop): retry failed transcriptions from history
❌ feat(desktop): retry failed transcriptions from history with saved audio and timestamp preservation

Better:
feat(desktop): retry failed transcriptions from history

Failed transcriptions can now be retried from the history
panel. The retry uses saved audio and preserves the
original timestamp.
```

### No Period

Subject ends without trailing period:

```
✅ feat(desktop): retry failed transcriptions
❌ feat(desktop): retry failed transcriptions.
```

---

## 5. Body Guidelines

Optional body provides technical context for developers.

### Rules

- Separate from subject with blank line
- Use imperative mood consistently
- Explain implementation details (subject already covers user impact)
- Wrap at 72 characters
- One paragraph per thought

### Example

```
feat(desktop): retry failed transcriptions from history

Failed transcriptions can now be retried from the history
panel. The retry uses saved audio and preserves the
original timestamp.

Implementation stores audio buffer on failure and adds a
retry button to failed history entries. Retry flow
reuses the existing transcription pipeline.
```

---

## 6. Footer Guidelines

Optional footer for issues, breaking changes, or attribution.

### Issue References

```
feat(desktop): retry failed transcriptions from history

Closes #123
Refs #456
```

### Breaking Changes

```
refactor(desktop): faster model loading on startup

BREAKING CHANGE: Custom model paths in settings are now
resolved relative to the app directory, not the home directory.
Users with custom paths may need to update their config.
```

---

## 7. Anti-Patterns

| Anti-Pattern | Fix |
|--------------|-----|
| No type (`Update CNAME`) | `chore(website): update CNAME` |
| Wrong type (`fix: add new feature`) | `feat(desktop): add new feature` |
| Past tense (`added retry`) | `add retry` |
| Capitalized subject (`Add retry`) | `add retry` |
| Trailing period (`add retry.`) | `add retry` |
| Vague subject (`fix bug`) | `fix(desktop): prevent recording truncation on long sessions` |
| Too technical (`fix audio chunk boundary`) | `fix(desktop): prevent long recordings from being truncated` |
| Implementation-focused (`refactor engine trait`) | `refactor(desktop): faster model loading on startup` |
| Invented scope (`feat(stt): ...`) | `feat(desktop): ...` |
| Too long subject (>72 chars) | Move detail to body |
| Non-English (`添加重试功能`) | `feat(desktop): retry failed transcriptions` |
| Scope for docs (`docs(context): ...`) | `docs: ...` (omit scope for repo-wide) |

---

## 8. Verification Checklist

- [ ] Type matches change category (Section 2)
- [ ] Scope is `desktop`, `website`, or omitted (Section 3)
- [ ] Subject describes user-facing impact, not implementation
- [ ] Subject uses imperative mood
- [ ] Subject starts lowercase
- [ ] Subject ≤72 characters
- [ ] Subject ends without period
- [ ] English only
- [ ] Body (if present) explains technical details
- [ ] Footer (if present) references issues or breaking changes

---

## 9. Commit Message Examples

### Good Examples (User-Facing)

```
feat(desktop): retry failed transcriptions from history
feat(desktop): cancel recording with ESC key
feat(desktop): support Fn key in custom shortcuts
fix(desktop): prevent long recordings from being truncated
fix(desktop): eliminate white flash on app startup
refactor(desktop): faster model loading on startup
chore: bump version to 0.3.0
docs: add v0.3 release notes
ci: add macOS binary pre-signing for smoother install
```

### Anti-Examples (Too Technical → User-Facing)

```
❌ fix audio chunk boundary handling
✅ fix(desktop): prevent long recordings from being truncated

❌ feat transcription retry with saved audio lookup
✅ feat(desktop): retry failed transcriptions from history

❌ refactor unify engine trait and lazy load
✅ refactor(desktop): faster model loading on startup

❌ fix return committed transcript from finish()
✅ fix(desktop): ensure transcription result appears after recording stops
```