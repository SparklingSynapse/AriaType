# Conventions

## Purpose

This directory consolidates all coding conventions, design system rules, and style guides.

## Principle

Conventions that aren't mechanically enforced will drift. If it matters, encode it in a linter or CI check.

## Index

- `design-system.md` — Desktop UI design tokens, components, patterns (canonical source)
- `website-design-system.md` — Website design tokens, components, patterns
- Rust conventions enforced by: `cargo clippy --all-features -- -D warnings` + `cargo fmt`
- TypeScript conventions enforced by: TypeScript strict mode + oxlint

## Rust Conventions

Key rules (not duplicating clippy):

- All identifiers, comments, doc strings in English
- `thiserror` for library errors, `anyhow` for application errors
- `tracing` for logging (never `println!`/`eprintln!`)
- Prefer `for` loops with mutable accumulators over complex iterator chains
- Newtypes over primitives (`UserId(u64)` not `u64`)
- Enums for state machines, not boolean flags

## TypeScript/React Conventions

- React 19 functional components with hooks
- All user-facing text → i18n keys
- `@/` path alias for imports
- Prefer stable UI state over aggressive fast updates
- Never call raw `invoke()` — always through `src/lib/tauri.ts`
