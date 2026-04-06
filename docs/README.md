# AriaType Documentation

**Philosophy**: Progressive disclosure, not encyclopedia. This index gives agents and developers a map, not a 1000-page instruction manual. Follow links for depth; stay shallow until you need to go deep.

## Project Principles

- Privacy comes first: voice data stays on-device by default, and cloud features are opt-in.
- Accuracy wins over speed: prefer correct, stable behavior over lower latency.
- Keep knowledge repository-local: key rules, specs, and architecture decisions must live in versioned docs.
- Keep one canonical source: avoid duplicate documents and put durable knowledge where readers naturally enter.

## Entry Points

| Document | Role |
|----------|------|
| [`../AGENTS.md`](../AGENTS.md) | Execution contract: rules, verification commands, product constraints |
| [`guides/onboarding.md`](./guides/onboarding.md) | Contributor/agent orientation: architecture, workflows, key files |
| [`architecture/README.md`](./architecture/README.md) | System map: domains, layers, and boundaries |
| [`architecture/decisions/README.md`](./architecture/decisions/README.md) | Why major architectural decisions were made |

## Canonical Sources

- Execution constraints: [`../AGENTS.md`](../AGENTS.md)
- Architectural rationale: [`architecture/decisions/README.md`](./architecture/decisions/README.md)
- Contracts and invariants: [`spec/`](./spec/) and [`architecture/data-flow.md`](./architecture/data-flow.md)

---

## Start Here

| Document | Description |
|----------|-------------|
| [`../AGENTS.md`](../AGENTS.md) | Agent operating contract, rules, verification commands — **read first** |
| [`guides/onboarding.md`](./guides/onboarding.md) | New contributor/agent onboarding — **read second** |
| [`architecture/README.md`](./architecture/README.md) | System architecture, domain map, package layering — **read third** |

---

## Architecture

| Document | Description |
|----------|-------------|
| [`architecture/README.md`](./architecture/README.md) | System architecture, domain map, package layering |
| [`architecture/layers.md`](./architecture/layers.md) | Layer model, dependency rules, boundary enforcement |
| [`architecture/data-flow.md`](./architecture/data-flow.md) | Core pipeline data flow and state machines |
| [`architecture/decisions/README.md`](./architecture/decisions/README.md) | Architecture decision records (ADRs) |

---

## Specifications

| Document | Description |
|----------|-------------|
| [`spec/logs.md`](./spec/logs.md) | Logging standard, structured fields, anti-patterns |
| [`spec/testing.md`](./spec/testing.md) | Test pyramid, coverage gates, verification commands |
| [`spec/engine-api-contract.md`](./spec/engine-api-contract.md) | STT/Polish engine API contracts, unified interfaces |

---

## Conventions

| Document | Description |
|----------|-------------|
| [`conventions/rust-style.md`](./conventions/rust-style.md) | Rust coding style (project-specific patterns) |
| [`conventions/typescript-style.md`](./conventions/typescript-style.md) | TypeScript/React coding style (project-specific patterns) |
| [`conventions/design-system.md`](./conventions/design-system.md) | Desktop UI design tokens, components (canonical source) |
| [`conventions/website-design-system.md`](./conventions/website-design-system.md) | Website UI design tokens, components |

---

## Guides

| Document | Description |
|----------|-------------|
| [`guides/onboarding.md`](./guides/onboarding.md) | New contributor/agent onboarding |
| [`guides/testing.md`](./guides/testing.md) | How to write and run tests |
| [`guides/debugging.md`](./guides/debugging.md) | Log investigation, crash reports, root cause analysis |
| [`guides/adding-stt-provider.md`](./guides/adding-stt-provider.md) | Adding a new STT provider |
| [`guides/adding-polish-provider.md`](./guides/adding-polish-provider.md) | Adding a new Polish provider |

---

## Reference

| Document | Description |
|----------|-------------|
| [`reference/providers/stt.md`](./reference/providers/stt.md) | STT provider API reference (Volcengine, OpenAI, Deepgram, etc.) |
| [`reference/providers/polish.md`](./reference/providers/polish.md) | Polish provider API reference (Anthropic, OpenAI, etc.) |

---

## Feature Specifications

Versioned feature specs with verification status. Each lives under `feat/<name>/<version>/prd/erd.md`.

| Feature | Version | Status |
|---------|---------|--------|
| [Home Dashboard Redesign](./feat/home-dashboard/0.1.0/prd/erd.md) | 0.1.0 | Active |
| [Website Homepage Redesign](./feat/website-homepage/0.1.0/prd/erd.md) | 0.1.0 | Active |
| [Cloud Service Tab UI](./feat/cloud-service/1.0.0/prd/erd.md) | 1.0.0 | Active |

---

## Plans

Active and completed execution plans.

| Location | Description |
|----------|-------------|
| [`plans/active/`](./plans/active/) | Active execution plans |
| [`plans/completed/README.md`](./plans/completed/README.md) | Completed execution plans index |
| [`plans/README.md`](./plans/README.md) | Plans index, lifecycle, and format |

---

## Quality

| Document | Description |
|----------|-------------|
| [`quality/README.md`](./quality/README.md) | Quality grades by domain (A-D scale, 24 domains) |
| [`quality/gardening.md`](./quality/gardening.md) | Doc gardening process, freshness verification |

---

## "I Want To..."

| Scenario | Go to |
|----------|-------|
| Understand agent rules | [`AGENTS.md`](../AGENTS.md) |
| Understand why the system works this way | [`architecture/decisions/README.md`](./architecture/decisions/README.md) |
| Onboard as new contributor/agent | [`guides/onboarding.md`](./guides/onboarding.md) |
| Set up dev environment | [`apps/desktop/CONTRIBUTING.md`](../apps/desktop/CONTRIBUTING.md) |
| Understand system architecture | [`architecture/README.md`](./architecture/README.md) |
| Add a new STT provider | [`guides/adding-stt-provider.md`](./guides/adding-stt-provider.md) |
| Add a new Polish provider | [`guides/adding-polish-provider.md`](./guides/adding-polish-provider.md) |
| Write and run tests | [`guides/testing.md`](./guides/testing.md) |
| Understand logging requirements | [`spec/logs.md`](./spec/logs.md) |
| Debug a production issue | [`guides/debugging.md`](./guides/debugging.md) |
| Look up provider API details | [`reference/`](./reference/README.md) |
| Understand a feature spec | `feat/<name>/<version>/prd/erd.md` |
| Add a new feature | [`AGENTS.md`](../AGENTS.md) rule 1 (Spec-first) |
| Maintain documentation freshness | [`quality/gardening.md`](./quality/gardening.md) |

---

## Doc Gardening

Docs grow stale. Keep them fresh:

- **On feature completion**: Update `feat/<name>/<version>/prd/erd.md` verification status
- **On architecture change**: Update `architecture/README.md` and relevant ADRs
- **On convention change**: Update `conventions/` docs
- **On refactor/migration**: Run a doc freshness pass across affected files and indexes
- **Monthly**: Skim this index, mark missing docs, prune empty sections
- **Full process**: [`quality/gardening.md`](./quality/gardening.md)

---

## Directory Map

```
docs/
├── README.md                          # This index
├── architecture/
│   ├── README.md                      # System architecture overview
│   ├── layers.md                      # Layer model and dependency rules
│   ├── data-flow.md                   # Core pipeline data flow and state machines
│   └── decisions/                     # Architecture Decision Records
│       ├── README.md                  # ADR index
│       ├── 001-unified-state-layer.md
│       ├── 002-nostream-volcengine.md
│       ├── 003-dual-layer-text-injection.md
│       └── 004-engine-trait-separation.md
│
├── spec/
│   ├── logs.md                        # Logging standard
│   ├── testing.md                     # Test pyramid and coverage gates
│   └── engine-api-contract.md         # Engine API contract testing
│
├── conventions/
│   ├── README.md                      # Conventions index
│   ├── rust-style.md                  # Rust coding style
│   ├── typescript-style.md            # TypeScript/React coding style
│   ├── design-system.md               # Desktop design system (canonical)
│   └── website-design-system.md       # Website design system
│
├── guides/
│   ├── README.md                      # Guides index
│   ├── onboarding.md                  # New contributor/agent onboarding
│   ├── testing.md                     # How to write and run tests
│   ├── debugging.md                   # Debugging and log investigation
│   ├── adding-stt-provider.md         # Adding new STT providers
│   └── adding-polish-provider.md      # Adding new Polish providers
│
├── reference/
│   ├── README.md                      # Reference index
│   └── providers/
│       ├── stt.md                     # STT provider API reference
│       └── polish.md                  # Polish provider API reference
│
├── feat/
│   ├── README.md                      # Feature specs index
│   ├── home-dashboard/0.1.0/prd/erd.md
│   ├── website-homepage/0.1.0/prd/erd.md
│   └── cloud-service/1.0.0/prd/erd.md
│
├── plans/
│   ├── README.md                      # Plans index and lifecycle
│   ├── active/                        # Active execution plans
│   └── completed/
│       └── README.md                  # Completed execution plans index
│
├── quality/
│   ├── README.md                      # Quality grades by domain
│   └── gardening.md                   # Doc gardening process
│
└── decisions/                         # Legacy ADR location (points to architecture/decisions/)
    └── README.md
```
