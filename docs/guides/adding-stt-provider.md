# Adding a Cloud STT Provider

This guide walks through adding a new cloud speech-to-text provider to the streaming engine framework.

## When to Read This

- Read [`../../AGENTS.md`](../../AGENTS.md) for execution constraints, verification expectations, and the default iteration loop
- Read [`../reference/providers/stt.md`](../reference/providers/stt.md) for provider-specific API details and existing conventions
- Read [`../architecture/data-flow.md`](../architecture/data-flow.md) when provider behavior affects pipeline contracts or state transitions
- Read this guide for the concrete integration steps inside the current STT engine architecture

## Overview

All STT engines (local and cloud) implement the unified `SttEngine` trait. Cloud providers integrate through the `StreamingSttClient` enum, which implements `SttEngine` and delegates to per-provider WebSocket clients.

## Step 1: Implement the Trait

Create `stt_engine/cloud/new_provider.rs`:

```rust
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::stt_engine::traits::{EngineType, PartialResultCallback, SttEngine, SttContext};
use crate::commands::settings::CloudSttConfig;

pub struct NewProviderEngine {
    partial_callback: Option<PartialResultCallback>,
    audio_sender: Option<mpsc::Sender<Vec<i16>>>,
    // Provider-specific fields
}

impl NewProviderEngine {
    pub fn new(config: CloudSttConfig, language: Option<&str>, context: SttContext) -> Self {
        Self {
            partial_callback: None,
            audio_sender: None,
        }
    }
}

#[async_trait]
impl SttEngine for NewProviderEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Cloud
    }

    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        // Forward to WebSocket (streaming) or buffer (non-streaming)
        todo!("Implement chunk forwarding")
    }

    async fn finish(&self) -> Result<String, String> {
        // Send EOS and return final transcription
        todo!("Implement session finalization")
    }

    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.partial_callback = Some(callback);
    }
}
```

## Step 2: Add to Client Enum

In `stt_engine/cloud/mod.rs`:

```rust
pub enum StreamingSttClient {
    Volcengine(VolcengineStreamingEngine),
    NewProvider(NewProviderEngine),
    // ... existing variants
}
```

## Step 3: Register in Engine

In `stt_engine/cloud/engine.rs`, add to `CloudSttEngine::new()`:

```rust
impl CloudSttEngine {
    pub fn new() -> Self {
        Self {
            client: if config.provider_type == "new-provider" {
                StreamingSttClient::NewProvider(NewProviderEngine::new(config))
            } else {
                // existing providers
            },
        }
    }
}
```

## Step 4: Frontend Configuration

Add provider option in `src/lib/tauri.ts`:

```typescript
export interface CloudSttConfig {
  provider_type: 'volcengine' | 'new-provider' | 'openai';
  // ... other fields
}
```

Update UI dropdown with the new provider.

## Step 5: i18n Keys

Add provider name to all 10 locale files in `src/i18n/locales/`:

```json
{
  "provider.new_provider": "New Provider",
  "settings.cloud_stt.provider.new_provider": "New Provider STT"
}
```

Supported locales: `de`, `en`, `es`, `fr`, `it`, `ja`, `ko`, `pt`, `ru`, `zh`.

## Step 6: Contract Tests

In `tests/cloud_provider_api_test.rs`:

```rust
#[tokio::test]
async fn test_stt_new_provider_schema() {
    let config = CloudSttConfig {
        enabled: true,
        provider_type: "new-provider".to_string(),
        api_key: mock_credentials::API_KEY.to_string(),
        app_id: mock_credentials::APP_ID.to_string(),
        base_url: "wss://api.newprovider.com/stt".to_string(),
        model: "default".to_string(),
        language: "en".to_string(),
    };

    // Use UnifiedEngineManager to create the engine
    let mut engine = StreamingSttClient::new(config, Some("en"), SttContext::default()).unwrap();
    engine.connect().await.unwrap();

    // Send a chunk and finish — expect auth error from real endpoint
    engine.send_chunk(vec![0i16; 512]).await.unwrap_err();
    // Auth error (401/403) proves endpoint URL and request format are correct
}
```

## Step 7: Integration Tests

Add pipeline tests in `src-tauri/tests/` that exercise the full transcription flow with mock audio data.

## Verification Checklist

- [ ] Trait implementation compiles
- [ ] Client enum builds
- [ ] Engine registration works
- [ ] Frontend renders new option
- [ ] All 10 locales have keys
- [ ] Contract test runs and gets auth error
- [ ] Integration test passes
