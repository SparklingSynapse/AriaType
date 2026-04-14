# Feature Specifications

Feature specs are versioned delivery contracts. Each spec at `context/feat/[name]/[semver]/prd/erd.md` defines:
- Problem statement
- Goal and acceptance criteria
- Data contract
- BDD scenarios
- Verification requirements

## When to Read This

- Read [`../README.md`](../README.md) for document routing and canonical sources
- Read [`../../AGENTS.md`](../../AGENTS.md) for the spec-first rule and execution constraints
- Read this directory when the question is "what exactly are we building and how do we prove it?"
- Do not use feature specs to explain architecture rationale after the fact; capture durable cross-cutting decisions in ADRs

## Active Features

| Feature | Version | Status | Spec |
|---------|---------|--------|------|
| Home Dashboard Redesign | 0.1.0 | Active | [erd.md](./home-dashboard/0.1.0/prd/erd.md) |
| Website Homepage Redesign | 0.1.0 | Active | [erd.md](./website-homepage/0.1.0/prd/erd.md) |
| Cloud Service Tab UI | 1.0.0 | Active | [erd.md](./cloud-service/1.0.0/prd/erd.md) |

## Completed Features

(None yet)

## Spec Format

Every feature spec MUST include:
1. **Version**: Feature name + semver
2. **Problem**: What's wrong or missing
3. **Goal**: What success looks like
4. **First-Principles Model**: Core questions the feature answers
5. **Information Architecture**: Content structure
6. **Data Contract**: Required fields and types
7. **Acceptance Criteria**: Binary pass/fail conditions
8. **BDD Scenarios**: Given/When/Then for key flows
9. **Verification**: How to confirm it works

## Rules

- Features are driven by specs, not chat interpretation
- If a spec doesn't exist, create it first, then implement against it
- Each versioned increment is implemented, tested, and verified independently
- Specs are the source of truth for feature intent and scope
