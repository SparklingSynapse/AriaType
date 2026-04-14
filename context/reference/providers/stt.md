# STT Provider API Documentation

This document describes the available Speech-to-Text (STT) cloud providers, their purposes, and official documentation.

---

## Overview

AriaType supports multiple cloud STT providers to transcribe audio into text. Each provider has different characteristics in terms of latency, accuracy, cost, and language support.

---

## Provider Summary

| Provider | Type | Latency | Best For | Languages |
|----------|------|---------|----------|-----------|
| Volcengine Streaming | WebSocket Real-time | Low (~200ms) | Live dictation, real-time transcription | 60+ languages |
| Volcengine Flash | HTTP Batch | Medium | Short recordings, batch processing | 60+ languages |
| OpenAI Whisper | HTTP Batch | Medium | High accuracy transcription | 50+ languages |
| OpenAI Realtime | WebSocket Real-time | Low | Real-time with GPT-4o capabilities | Multiple |
| Deepgram | WebSocket Real-time | Very Low (~300ms) | Fast streaming, cost-effective | 30+ languages |
| Custom Endpoint | HTTP/WebSocket | Varies | Self-hosted or custom STT services | Varies |

---

## Volcengine Streaming

### Description
Real-time streaming STT via WebSocket. Provides partial results during speech, ideal for live dictation scenarios.

### Purpose
- Real-time voice input
- Live transcription with immediate feedback
- Continuous speech recognition

### API Endpoint
```
wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream
```

### Configuration Required
- **App ID**: Application identifier from Volcengine Console
- **Access Token**: Authentication token (may expire, needs refresh)
- **Resource ID** (optional): Default `volc.bigasr.sauc.duration`

### Official Documentation
- [Volcengine Speech-to-Text Console](https://console.volcengine.com/sami)
- [API Documentation](https://www.volcengine.com/context/6561/79817)
- [Streaming Protocol](https://www.volcengine.com/context/6561/79818)

### Key Features
- Partial results during speech
- Language auto-detection
- Punctuation and formatting
- Low latency (~200ms chunk processing)

### STT Output Principle: Raw & Accurate

> **STT should output raw, unprocessed transcription.** The Polish engine handles formatting (ITN, punctuation, deduplication). STT's job is accurate phonetic transcription, not polished text.

**Volcengine configuration:**
```json
{
  "enable_itn": true,     // Numbers: "一九七八" → "1978" — Polish engine handles formatting
  "enable_punc": true,   // Punctuation: "今天天气好" → "今天天气好。" — Polish engine handles formatting
  "enable_ddc": false    // Disfluency/deduplication — OFF: preserve raw output
}
```

---

## Volcengine Flash

### Description
HTTP-based batch STT for short audio files. Lower cost than streaming, suitable for offline processing.

### Purpose
- Short audio transcription (< 60 seconds)
- Batch processing of recordings
- Cost-sensitive scenarios

### API Endpoint
```
https://openspeech.bytedance.com/api/v3/auc/bigmodel/recognize/flash
```

### Configuration Required
- **App ID**: Application identifier
- **Access Token**: Authentication token
- **Resource ID** (optional): Model identifier

### Official Documentation
- [Volcengine Flash API](https://www.volcengine.com/context/6561/79819)

### Key Features
- Lower cost than streaming
- No real-time results
- Suitable for batch processing
- Good for short recordings

---

## OpenAI Whisper

### Description
OpenAI's Whisper model via batch API. High accuracy transcription with support for 50+ languages.

### Purpose
- High-accuracy transcription
- Multi-language support
- Scenarios where latency is not critical

### API Endpoint
```
https://api.openai.com/v1/audio/transcriptions
```

### Configuration Required
- **API Key**: OpenAI API key from platform.openai.com
- **Model** (optional): Default `whisper-1`

### Official Documentation
- [OpenAI Audio API](https://platform.openai.com/context/api-reference/audio)
- [Whisper Model](https://platform.openai.com/context/models/whisper)

### Key Features
- High accuracy
- 50+ language support
- Translation capability (to English)
- Price: $0.006/minute

---

## OpenAI Realtime

### Description
OpenAI's Realtime API with GPT-4o for low-latency speech-to-text with advanced capabilities.

### Purpose
- Real-time transcription with AI capabilities
- Voice-to-voice applications
- Advanced audio understanding

### API Endpoint
```
wss://api.openai.com/v1/realtime
```

### Configuration Required
- **API Key**: OpenAI API key with Realtime API access
- **Model**: `gpt-4o-realtime-preview-2026-12-17`

### Official Documentation
- [OpenAI Realtime API](https://platform.openai.com/context/api-reference/realtime)
- [Realtime API Guide](https://platform.openai.com/context/guides/realtime)

### Key Features
- Very low latency
- GPT-4o powered understanding
- Function calling support
- Audio output capability

---

## Deepgram

### Description
Deepgram's streaming STT via WebSocket. Fast, accurate, and cost-effective for real-time transcription.

### Purpose
- Fast streaming transcription
- Cost-effective real-time STT
- High-volume applications

### API Endpoint
```
wss://api.deepgram.com/v1/listen
```

### Configuration Required
- **API Key**: Deepgram API key from console.deepgram.com
- **Model** (optional): Default `nova-2`
- **Language** (optional): Language code (e.g., `en-US`, `zh-CN`)

### Official Documentation
- [Deepgram API Reference](https://developers.deepgram.com/api-reference/)
- [Streaming STT](https://developers.deepgram.com/documentation/features/streaming/)
- [Console](https://console.deepgram.com/)

### Key Features
- Very fast (~300ms latency)
- Nova-2 model for high accuracy
- Interim results
- Smart formatting and punctuation
- Cost-effective pricing

---

## Custom Endpoint

### Description
OpenAI-compatible custom STT endpoint for self-hosted or third-party STT services.

### Purpose
- Self-hosted STT services
- Third-party STT providers with OpenAI-compatible API
- Custom STT implementations

### Configuration Required
- **API Key**: Authentication key for your service
- **Base URL**: Your STT API endpoint
- **Model** (optional): Model identifier

### Use Cases
- Self-hosted Whisper models
- Azure Speech Services
- Google Cloud Speech-to-Text
- Custom STT implementations

### Example Configuration
```json
{
  "provider_type": "custom",
  "api_key": "your-api-key",
  "base_url": "https://your-stt-service.com/v1/audio/transcriptions",
  "model": "whisper-large-v3"
}
```

---

## Provider Selection Guide

### Choose Volcengine Streaming when:
- You need real-time transcription for live dictation
- Low latency is critical
- You're primarily targeting Chinese users

### Choose Volcengine Flash when:
- You have short recordings (< 60 seconds)
- Cost is a primary concern
- Real-time results are not needed

### Choose OpenAI Whisper when:
- Accuracy is the top priority
- You need multi-language support
- Latency is acceptable

### Choose OpenAI Realtime when:
- You need advanced AI capabilities
- Voice-to-voice interaction is required
- Budget allows for premium service

### Choose Deepgram when:
- You need fast, cost-effective streaming STT
- High volume with good accuracy
- English-language focus

### Choose Custom Endpoint when:
- You have self-hosted STT infrastructure
- You need a provider not directly supported
- You want full control over STT pipeline