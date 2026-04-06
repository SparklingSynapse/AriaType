# Doc Gardening

Documentation rots. This document defines the mechanical process for keeping docs fresh, accurate, and consistent with the codebase.

---

## Why Doc Gardening Matters

From Harness Engineering: "A monolithic manual turns into a graveyard of stale rules. Agents can't tell what's still true, humans stop maintaining it, and the file quietly becomes an attractive nuisance."

Technical debt in documentation is like a high-interest loan. Pay it down continuously in small increments rather than letting it compound.

---

## Gardening Triggers

### On Every Feature Completion

- [ ] Update `feat/<name>/<version>/prd/erd.md` verification status
- [ ] Update `quality/README.md` domain grade if affected
- [ ] Verify cross-links from `docs/README.md` are still correct
- [ ] Check if any ADR was implicitly created or invalidated

### On Every Architecture Change

- [ ] Update `architecture/README.md` domain map
- [ ] Update `architecture/layers.md` if dependency rules changed
- [ ] Update `architecture/data-flow.md` if data contracts changed
- [ ] Record new ADR if a significant decision was made

### On Every Convention Change

- [ ] Update `conventions/README.md` and relevant style doc
- [ ] Verify linter rules reflect the convention (if it matters, encode it)

### On Refactor or Migration

- [ ] Run a doc freshness pass across affected files and indexes
- [ ] Update `plans/` if active or completed plans are affected
- [ ] Check `guides/` for outdated step-by-step instructions

### Monthly Cadence

- [ ] Skim `docs/README.md` index, mark missing docs, prune empty sections
- [ ] Verify `docs/README.md` still declares canonical sources for execution constraints, architectural rationale, and contracts
- [ ] Verify all "Last Verified" dates in `quality/README.md` are current for the latest monthly review
- [ ] Scan for broken cross-links between documents

---

## Freshness Verification

### Mechanical Checks

| Check | Method | Frequency |
|-------|--------|-----------|
| File existence | All files referenced in indexes exist | Every PR |
| Cross-link validity | All `[relative links]` resolve | Every PR |
| Canonical routing | `docs/README.md` includes `## Canonical Sources` and domain indexes include `## When to Read This` | Every PR |
| Quality date freshness | `Last Verified` is current for the latest monthly review | Monthly |
| Feature status accuracy | `Active` features have corresponding code | Monthly |
| ADR status | No `Proposed` ADRs older than 30 days | Monthly |
| Spec-code alignment | Spec contracts match actual TypeScript interfaces | On refactor |

### Stale Doc Indicators

A document is stale if ANY of these are true:

1. **References deleted files or modules** — paths that no longer exist
2. **Describes removed behavior** — e.g., a setting that was replaced
3. **Contradicts the code** — e.g., spec says "required" but code treats as optional
4. **Last verified is older than the latest monthly review** — quality grades are uncertain
5. **Empty sections** — placeholders that were never filled
6. **Broken links** — references to files that moved or were deleted

---

## Gardening Process

### For Humans

1. During code review, flag doc updates needed in the PR
2. On feature merge, verify spec status is updated
3. On Friday: 15-minute skim of `docs/README.md` for obvious staleness

### For Agents

Agents performing doc gardening should:

1. **Scan**: Read each document in `docs/` and check references
2. **Verify**: Confirm referenced files exist and content matches
3. **Report**: Prepare fix-up changes for:
   - Broken links
   - Stale quality dates
   - Incorrect file paths
   - Empty sections that should have content
4. **Do NOT**: Rewrite content, change conventions, or add new sections without human approval

### Agent Gardening Prompt Template

```
Scan docs/ directory for stale documentation:
1. Read every .md file in docs/ and its subdirectories
2. For each file, check that all [relative links] point to existing files
3. For quality/README.md, check that "Last Verified" dates are current for the latest monthly review
4. For feat/README.md, verify that "Active" features have corresponding code
5. For plans/README.md, verify active and completed indexes match the actual directory state
6. For guides/README.md, verify that all listed files exist
7. Report findings and prepare fix-up changes for any issues found
```

---

## Doc Structure Invariants

These rules are mechanically enforceable:

1. **Every top-level docs domain has an index** — `docs/*/README.md` exists for each semantic domain directory
2. **Each domain index links to its canonical children** — indexes cover the files or sub-indexes humans are expected to enter through
3. **Domain indexes declare entry scope** — top-level indexes include a short `When to Read This` section so readers know what the directory does and does not answer
4. **Root routing stays explicit** — `docs/README.md` declares canonical sources for execution constraints, architectural rationale, and contracts
5. **No orphan domain documents** — every long-lived `.md` file under `docs/` is linked from at least one index or redirect document
6. **No duplicate content** — each piece of knowledge lives in exactly one place (canonical source)
7. **Progressive disclosure** — index files are maps (pointers), while deep content lives in leaf documents
8. **Container directories are exempt** — version folders like `feat/<name>/<version>/prd/` and storage folders like `plans/active/` do not need standalone indexes unless they become human entry points

---

## Quality Grade Doc Update Rules

When updating `quality/README.md`:

| Grade Change | Action Required |
|--------------|----------------|
| Any → A | Must have automated test evidence |
| A → B | Document what regressed |
| B → C | Create plan to restore to B |
| Any → D | Create plan immediately, mark as priority |
| Date only | Update "Last Verified" with today's date |
