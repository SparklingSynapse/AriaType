# AriaType Documentation

**Philosophy**: Progressive disclosure, not encyclopedia. This index gives agents and developers a map, not a 1000-page instruction manual. Follow links for depth; stay shallow until you need to go deep.

---

## Core References

| Document | Description |
|----------|-------------|
| [AGENTS.md](../AGENTS.md) | Agent operating contract, rules, verification standards |
| [beliefs.md](./beliefs.md) | Core beliefs and non-negotiable constraints |
| [CONTRIBUTING (desktop)](../apps/desktop/CONTRIBUTING.md) | Desktop app development guide (React + Rust/Tauri) |
| [CONTRIBUTING (shared)](../packages/shared/CONTRIBUTING.md) | Shared TypeScript types and constants |
| [CONTRIBUTING (website)](../packages/website/CONTRIBUTING.md) | Next.js marketing site development |
| [STT Engine Guide](../apps/desktop/src-tauri/CONTRIBUTING.md) | STT engine architecture and provider integration |

---

## Architecture

| Document | Description |
|----------|-------------|
| [architecture/README.md](./architecture/README.md) | System architecture, layers, data flow |
| [spec/logs.md](./spec/logs.md) | Logging standard, structured fields, anti-patterns |
| [spec/testing.md](./spec/testing.md) | Test pyramid, coverage gates, verification commands |
| [spec/engine-api-contract.md](./spec/engine-api-contract.md) | STT/Polish engine API contracts, unified interfaces |

---

## Conventions

| Document | Description |
|----------|-------------|
| [conventions/README.md](./conventions/README.md) | Coding conventions, naming, Rust/TypeScript style |
| [conventions/design-system.md](./conventions/design-system.md) | Design tokens, component patterns, UI guidelines |

---

## Feature Specifications

Versioned feature specs with verification status. Each lives under `feat/<name>/<version>/prd/erd.md`.

| Feature | Status |
|---------|--------|
| [home-dashboard](./feat/home-dashboard/0.1.0/prd/erd.md) | In progress |
| [website-homepage](./feat/website-homepage/0.1.0/prd/erd.md) | In progress |
| [stt-provider](./plans/feat/stt-provider/api.md) | Planned |
| [cloud-service](./plans/feat/cloud-service/erd.md) | Planned |
| [llm-provider](./plans/feat/llm-provider/api.md) | Planned |

---

## Plans

Active and completed execution plans.

| Plan | Description |
|------|-------------|
| [Logging standardization](./plans/2026-04-03-001-fix-logging-standardization-plan.md) | Fix logging standard violations |

---

## Guides

| Document | Description |
|----------|-------------|
| [guides/debugging.md](./guides/debugging.md) | Log investigation, crash reports, Tauri debugging |
| [guides/adding-stt-provider.md](./guides/adding-stt-provider.md) | Adding new STT/Polish providers |

---

## Quality & Decisions

| Document | Description |
|----------|-------------|
| [quality/README.md](./quality/README.md) | Quality grades by domain (STT accuracy, stability, UX, speed) |
| [decisions/README.md](./decisions/README.md) | Architecture decision records (ADRs) |

---

## "I Want To..."

| Scenario | Go to |
|----------|-------|
| Understand agent rules and constraints | [AGENTS.md](../AGENTS.md) |
| Understand why the system works this way | [beliefs.md](./beliefs.md) |
| Set up dev environment | [CONTRIBUTING (desktop)](../apps/desktop/CONTRIBUTING.md) |
| Add a new STT provider | [STT Engine Guide](../apps/desktop/src-tauri/CONTRIBUTING.md) + [guides/adding-stt-provider.md](./guides/adding-stt-provider.md) |
| Understand logging requirements | [spec/logs.md](./spec/logs.md) |
| Run tests and check coverage | [spec/testing.md](./spec/testing.md) |
| Debug a production issue | [guides/debugging.md](./guides/debugging.md) |
| Understand a feature spec | `feat/<name>/<version>/prd/erd.md` |
| Add a new feature | Start with [AGENTS.md](../AGENTS.md) section 2.2 (Spec-Driven Feature Delivery) |

---

## Doc Gardening

Docs grow stale. Keep them fresh:

- **On feature completion**: Update `feat/<name>/<version>/prd/erd.md` verification status
- **On architecture change**: Update `architecture/README.md` and relevant ADRs
- **On convention change**: Update `conventions/README.md`
- **On refactor/migration**: Run `ce:compound-refresh` skill to audit and update stale docs
- **Monthly**: Skim this index, mark missing docs, prune empty sections

---

## Directory Map

```
docs/
├── README.md             # This index
├── beliefs.md            # Core beliefs and constraints
├── architecture/
│   ├── README.md         # System architecture overview
│   ├── layers.md         # Layer model and dependency rules
│   └── data-flow.md      # Core pipeline data flow and state machines
├── spec/
│   ├── logs.md           # Logging standard
│   ├── testing.md        # Test pyramid and coverage gates
│   └── engine-api-contract.md  # Engine API contract testing
├── conventions/
│   ├── README.md         # Coding conventions index
│   └── design-system.md  # Design tokens and component patterns
├── feat/
│   └── <feature>/<version>/prd/erd.md  # Versioned feature specs
├── plans/
│   └── feat/             # Feature implementation plans
├── guides/
│   ├── debugging.md      # Log investigation and debugging
│   └── adding-stt-provider.md  # Adding new STT providers
├── quality/
│   └── README.md         # Quality grades by domain
└── decisions/
    └── README.md         # Architecture decision records
```
