# Adding a Text Polish Provider

This guide walks through adding a new text polish provider to the polish engine framework.

## When to Read This

- Read [`../../AGENTS.md`](../../AGENTS.md) for execution constraints, verification expectations, and the default iteration loop
- Read [`../reference/providers/polish.md`](../reference/providers/polish.md) for provider-specific API details and existing conventions
- Read [`../architecture/data-flow.md`](../architecture/data-flow.md) when provider behavior affects pipeline contracts or state transitions
- Read this guide for the concrete integration steps inside the current polish engine architecture

## Overview

The polish engine uses a trait-based architecture. Providers implement `PolishEngine` and integrate into `UnifiedPolishManager`.

**Two provider types:**
- **Local providers**: Run on-device (e.g., Qwen, LFM) — add new module in `polish_engine/`
- **Cloud providers**: Call external APIs (e.g., Anthropic, OpenAI) — extend `CloudPolishEngine`

## Step 1: Implement the Trait

### For Local Providers

Create `polish_engine/new_provider/engine.rs`:

```rust
use crate::polish_engine::traits::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use async_trait::async_trait;

pub struct NewProviderEngine;

#[async_trait]
impl PolishEngine for NewProviderEngine {
    fn engine_type(&self) -> PolishEngineType {
        PolishEngineType::NewProvider
    }

    async fn polish(&self, request: PolishRequest) -> Result<PolishResult, String> {
        let t0 = std::time::Instant::now();
        // ... inference logic ...
        let total_ms = t0.elapsed().as_millis() as u64;
        Ok(PolishResult::new(result_text, PolishEngineType::NewProvider, total_ms))
    }
}
```

### For Cloud Providers

Extend `CloudPolishEngine` in `polish_engine/cloud/engine.rs`:

```rust
async fn call_new_provider_api(&self, system_prompt: &str, user_message: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": self.config.model,
        "prompt": format!("{}\n\n{}", system_prompt, user_message),
    });
    // Send request and parse response...
}
```

Add to provider switch in `polish()`:

```rust
let result = match self.config.provider_type.as_str() {
    "anthropic" => self.call_anthropic_api(&system_prompt, &input_text).await?,
    "openai" => self.call_openai_api(&system_prompt, &input_text).await?,
    "new-provider" => self.call_new_provider_api(&system_prompt, &input_text).await?,
    _ => self.call_openai_api(&system_prompt, &input_text).await?,
};
```

## Step 2: Add Engine Type to Enum

In `polish_engine/traits.rs` (for local providers):

```rust
pub enum PolishEngineType {
    Qwen, Lfm, Cloud, NewProvider,
}

impl PolishEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NewProvider => "new-provider",
            // ... existing variants
        }
    }
}
```

## Step 3: Register in Unified Manager

In `polish_engine/unified_manager.rs`:

```rust
engines.insert(
    PolishEngineType::NewProvider,
    Arc::new(new_provider::NewProviderEngine::new()),
);
```

Update `mod.rs` to export the new module:

```rust
pub mod new_provider;
pub use new_provider::NewProviderEngine;
```

## Step 4: Frontend Configuration

In `src/components/Home/cloud/CloudPolishSection.tsx`:

```typescript
type CloudPolishProvider = "anthropic" | "openai" | "new-provider" | "custom";

const POLISH_PROVIDERS = [
  { value: "anthropic", labelKey: "model.polish.cloud.providerAnthropic" },
  { value: "openai", labelKey: "model.polish.cloud.providerOpenAI" },
  { value: "new-provider", labelKey: "model.polish.cloud.providerNewProvider" },
];
```

## Step 5: i18n Keys

Add to all 10 locale files in `src/i18n/locales/`:

```json
"model.polish.cloud.providerNewProvider": "New Provider"
```

**Locales**: `de`, `en`, `es`, `fr`, `it`, `ja`, `ko`, `pt`, `ru`, `zh`.

## Step 6: Contract Tests

In `tests/cloud_polish_mock_test.rs`:

```rust
#[tokio::test]
async fn test_new_provider_request_format() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).mount(&mock_server).await;
    
    let config = CloudProviderConfig {
        provider_type: "new-provider".to_string(),
        api_key: "test_key".to_string(),
        base_url: format!("{}/v1/polish", mock_server.uri()),
        model: "model".to_string(),
        enable_thinking: false,
    };
    let engine = CloudPolishEngine::new(config);
    let result = engine.polish(request).await.expect("failed");
    assert!(!result.text.is_empty());
}
```

## Step 7: Integration Tests

In `tests/polish_engine_test.rs`:

```rust
#[tokio::test]
async fn test_new_provider_basic() {
    let engine = NewProviderEngine::new();
    let result = engine.polish(PolishRequest::new("test", "prompt", "en")).await.unwrap();
    assert_eq!(result.engine, PolishEngineType::NewProvider);
}
```

## Verification Checklist

- [ ] Trait implementation compiles (`cargo check`)
- [ ] Engine type added to enum (local) or provider switch (cloud)
- [ ] Unified manager registration works
- [ ] Frontend renders new provider option
- [ ] All 10 locales have i18n keys
- [ ] Mock server test passes (cloud)
- [ ] Integration test passes
- [ ] `cargo clippy --all-features -- -D warnings` passes
- [ ] `cargo test` passes

## Key Files Reference

| File | Purpose |
|------|---------|
| `polish_engine/traits.rs` | `PolishEngine` trait, `PolishEngineType`, request/result types |
| `polish_engine/cloud/engine.rs` | `CloudPolishEngine` for all cloud providers |
| `polish_engine/unified_manager.rs` | Engine lifecycle, caching |
| `src/lib/tauri.ts` | Frontend IPC types |
| `src/components/Home/cloud/CloudPolishSection.tsx` | Settings UI |
| `tests/cloud_polish_mock_test.rs` | Cloud API contract tests |

## Differences from STT Providers

| Aspect | STT | Polish |
|--------|-----|--------|
| Cloud architecture | Separate client per provider | Single `CloudPolishEngine` handles all |
| Trait methods | `transcribe()` or streaming lifecycle | Single `polish()` method |
| Engine types | Unified `SttEngine` (send_chunk + finish) | Single `PolishEngine` trait |
| Request flow | Audio → Text | Text → Text |

## Related Documentation

- [Engine API Contract Testing](../spec/engine-api-contract.md)
- [Adding STT Provider](./adding-stt-provider.md)
