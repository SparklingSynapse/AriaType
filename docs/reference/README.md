# Provider Reference

API reference documentation for cloud and local providers integrated into AriaType.

## When to Read This

- Read [`../README.md`](../README.md) for document routing and canonical sources
- Read [`../guides/`](../guides/README.md) for step-by-step integration or debugging workflows
- Read [`../spec/engine-api-contract.md`](../spec/engine-api-contract.md) for contract-testing policy and engine-level interface rules
- Read this directory when you need provider facts: endpoints, auth methods, models, limits, or request shapes

## Purpose

This directory contains stable API reference documentation for external providers. These docs support implementation and debugging; they are not plans or strategy documents.

## Provider Categories

| Category | Document | Description |
|----------|----------|-------------|
| **STT (Speech-to-Text)** | [providers/stt.md](./providers/stt.md) | All STT providers: Volcengine, OpenAI, Deepgram, ElevenLabs, Qwen Omni, Custom Endpoint |
| **Polish (Text Enhancement)** | [providers/polish.md](./providers/polish.md) | All text polishing providers: Anthropic, OpenAI, Qwen, Custom Endpoint |

## How These Docs Relate

| Want to... | Go to... |
|------------|----------|
| Understand engine API contracts | [spec/engine-api-contract.md](../spec/engine-api-contract.md) |
| Add a new STT provider | [guides/adding-stt-provider.md](../guides/adding-stt-provider.md) |
| Add a new Polish provider | [guides/adding-polish-provider.md](../guides/adding-polish-provider.md) |
| Look up provider-specific API details | [providers/](./providers/) |
| Understand provider selection logic | [architecture/data-flow.md](../architecture/data-flow.md) |

## Maintenance

- Update provider docs when API contracts change or new providers are added
- Verify provider URLs and authentication methods against actual provider documentation
- Keep feature comparison tables current with implemented functionality
