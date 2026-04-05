# Adding a Cloud STT Provider

This guide walks through adding a new cloud speech-to-text provider to the streaming engine framework.

## Overview

The streaming engine uses a trait-based architecture. Each provider implements `StreamingSttEngine` and integrates into `CloudSttEngine` through `StreamingSttClient` enum.

## Step 1: Implement the Trait

Create `stt_engine/cloud/new_provider.rs`:

```rust
use async_trait::async_trait;
use tokio::sync::Sender;

use crate::stt_engine::cloud::{StreamingSttEngine, PartialResultCallback};

pub struct NewProviderEngine {
    partial_callback: Option<PartialResultCallback>,
    // Provider-specific fields
}

impl NewProviderEngine {
    pub fn new(config: NewProviderConfig) -> Self {
        Self {
            partial_callback: None,
        }
    }
}

#[async_trait]
impl StreamingSttEngine for NewProviderEngine {
    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.partial_callback = Some(callback);
    }

    async fn connect(&mut self) -> Result<(), String> {
        // Establish WebSocket connection
    }

    async fn get_audio_sender(&self) -> Option<Sender<Vec<i16>>> {
        // Return audio sender channel
    }

    async fn finish(&self) -> Result<String, String> {
        // Send EOS and return transcription
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

    let engine = CloudSttEngine::new().unwrap();
    let result = engine.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // Auth error proves request format is correct
    assert!(err.contains("401") || err.contains("403"));
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
