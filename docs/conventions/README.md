# Conventions

## When to Read This

- Read [`../README.md`](../README.md) for document routing and canonical sources
- Read [`../../AGENTS.md`](../../AGENTS.md) only for execution constraints and verification rules
- Read this directory for coding style, design system rules, and repeatable project conventions
- Do not treat conventions as feature intent or architecture rationale; use specs and ADRs for those questions

## Purpose

This directory consolidates all coding conventions, design system rules, and style guides.

## Principle

Conventions that aren't mechanically enforced will drift. If it matters, encode it in a linter or CI check.

## Index

| Document | Description |
|----------|-------------|
| [`rust-style.md`](./rust-style.md) | Rust coding style — error handling, logging, types, async, tests |
| [`typescript-style.md`](./typescript-style.md) | TypeScript/React style — strict mode, hooks, IPC, i18n, Tailwind |
| [`design-system.md`](./design-system.md) | Desktop UI design tokens, components, patterns (canonical source) |
| [`website-design-system.md`](./website-design-system.md) | Website UI design tokens, components, patterns |

## Enforcement

| Layer | Tool | Scope |
|-------|------|-------|
| Rust | `cargo clippy --all-features -- -D warnings` + `cargo fmt` | All Rust code |
| TypeScript | TypeScript strict mode + oxlint | All TS/TSX code |
| i18n | `pnpm check:i18n` | All locale files (10 locales) |
