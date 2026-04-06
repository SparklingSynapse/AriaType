# Data Flow

## Primary User Workflow

```
User holds hotkey
  → GlobalShortcut registered (tray.rs → commands/)
  → Recording starts (audio/recorder.rs)
  → Audio captured (16kHz mono PCM)
  → If cloud STT: chunks streamed every 1s via WebSocket
  → If local STT: complete audio stored in AudioStorage
User releases hotkey
  → Recording stops
  → WAV file written
  → STT engine processes audio
  → Transcription result received
  → If polish enabled: text polished (local LLM or cloud API)
  → Final text injected at cursor (text_injector/)
  → Session recorded in history
```

## Engine Selection Flow

```
Settings
  → enabled?
  → cloud or local?

Local Engine Path:
  → Check downloaded model exists
  → Load model if not cached
  → Batch transcribe complete audio
  → Return TranscriptionResult

Cloud Engine Path:
  → Check credentials configured
  → Connect WebSocket (for streaming) or HTTP
  → Stream audio chunks (1s intervals for streaming STT)
  → Receive partial results
  → Receive final transcription
  → Return TranscriptionResult
```

## State Machine

```
RecordingState

Idle
  │
  │ hold hotkey
  ▼
Recording
  │
  │ release hotkey
  ▼
Processing
  │
  │ transcription complete
  ▼
Injecting
  │
  │ injection complete OR injection failed
  ▼
Idle

Error States:
  RecordingFailed     — mic access denied, device error
  TranscriptionFailed — engine error, invalid audio
  InjectionFailed    — clipboard error, focus error
```

## IPC Communication

### Frontend → Backend

All calls go through `src/lib/tauri.ts`:

```typescript
invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>
```

Key commands:
- `start_recording` — begin audio capture
- `stop_recording` — end capture and transcribe
- `get_settings` — retrieve current settings
- `update_settings` — persist settings changes
- `get_history` — retrieve transcription history

### Backend → Frontend

Events emitted through `events/` module:

| Event | Payload | Trigger |
|-------|---------|---------|
| `recording-state-changed` | `RecordingState` | State transitions |
| `audio-level` | `number` (0.0-1.0) | Level meter updates |
| `transcription-complete` | `TranscriptionResult` | Successful transcription |
| `transcription-error` | `string` (error message) | Engine failure |
| `model-download-progress` | `{ progress: number, model: string }` | Model download |
| `polish-progress` | `string` | Polish operation update |

Frontend subscribes via:

```typescript
listen<T>(event: string, handler: (event: T) => void): Promise<UnlistenFn>
```

## Data Contracts

Language fields use full IETF BCP 47 tags. Use `language-REGION` values such as `en-US`, `zh-CN`, `ja-JP`, and `ko-KR`, not bare language codes like `en` or `zh`.

### TranscriptionRequest

```typescript
interface TranscriptionRequest {
  audio_path: string;           // Path to WAV file
  language?: string;            // BCP 47 language tag (e.g., "zh-CN")
  cloud_config?: CloudSttConfig; // Cloud provider settings
}
```

### TranscriptionResult

```typescript
interface TranscriptionResult {
  text: string;                 // Final transcribed text
  engine_type: "whisper" | "sense_voice" | "volcengine" | "qwen_omni" | "elevenlabs" | "deepgram";
  duration_ms: number;          // Processing time
  audio_duration_ms?: number;   // Original audio length
}
```

### CloudSttConfig

```typescript
interface CloudSttConfig {
  provider_type: "volcengine" | "qwen_omni" | "elevenlabs" | "deepgram";
  api_key: string;
  app_id?: string;              // Volcengine requires this
  base_url: string;             // WebSocket or HTTP endpoint
  model: string;                // Provider-specific model name
  language?: string;            // BCP 47 language tag
}
```

### PolishResult

```typescript
interface PolishResult {
  polished_text: string;        // Refined text
  original_text: string;        // Original transcription
  duration_ms: number;          // Processing time
  engine_type: "lfm" | "qwen" | "anthropic" | "openai" | "custom";
}
```

## Audio Pipeline

```
Microphone (system audio capture)
  → Audio Recorder (16kHz mono PCM)
  → Level Meter (real-time amplitude)
  → VAD (voice activity detection, optional)
  → AudioStorage (in-memory buffer)
  → WAV Writer (file output)
  → STT Engine (transcription)
```

## Multi-Window Model

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Main      │     │    Pill     │     │    Toast    │
│  Window     │     │  (floating) │     │ (transient) │
│ main.tsx    │     │  pill.tsx   │     │  toast.tsx   │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                   settings context
```

- **Main window** — Settings dashboard, history browser
- **Pill** — Floating indicator during recording
- **Toast** — Transient notifications (errors, completion)
