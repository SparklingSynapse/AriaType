# LLM Provider API Documentation

This document describes the available Large Language Model (LLM) cloud providers for text polishing, their purposes, and official documentation.

---

## Overview

AriaType uses cloud LLMs to polish transcribed text, improving readability, grammar, and formatting. The polish engine supports multiple providers with different models and capabilities.

---

## Provider Summary

| Provider | Default Model | Best For | Pricing Model |
|----------|---------------|----------|---------------|
| Anthropic | Claude 3.5 Sonnet | High-quality writing, nuanced text improvement | Pay-per-token |
| OpenAI | GPT-4o | Fast polishing, good value | Pay-per-token |
| Custom | User-defined | Self-hosted or alternative LLMs | Varies |

---

## Anthropic (Claude)

### Description
Anthropic's Claude models for text polishing. Known for nuanced understanding and high-quality writing output.

### Purpose
- High-quality text polishing
- Nuanced grammar and style improvements
- Professional document refinement

### API Endpoint
```
https://api.anthropic.com/v1/messages
```

### Configuration Required
- **API Key**: Anthropic API key from console.anthropic.com
- **Model** (optional): Default `claude-3-5-sonnet-20241022`
- **Base URL** (optional): For custom endpoints

### Supported Models
- `claude-3-5-sonnet-20241022` (recommended)
- `claude-3-opus-20240229`
- `claude-3-sonnet-20240229`
- `claude-3-haiku-20240307`

### Official Documentation
- [Anthropic API Docs](https://docs.anthropic.com/claude/reference)
- [Messages API](https://docs.anthropic.com/claude/reference/messages_post)
- [Console](https://console.anthropic.com/)

### Key Features
- Excellent writing quality
- Strong instruction following
- Large context window (200K tokens)
- Thinking mode support (extended reasoning)

### Headers Required
```
x-api-key: <your-api-key>
anthropic-version: 2023-06-01
```

### Pricing
- Claude 3.5 Sonnet: $3/$15 per 1M tokens (input/output)
- Claude 3 Opus: $15/$75 per 1M tokens
- Claude 3 Haiku: $0.25/$1.25 per 1M tokens

---

## OpenAI (GPT)

### Description
OpenAI's GPT models for text polishing. Fast, reliable, and widely compatible.

### Purpose
- Fast text polishing
- General-purpose text improvement
- Cost-effective polishing at scale

### API Endpoint
```
https://api.openai.com/v1/chat/completions
```

### Configuration Required
- **API Key**: OpenAI API key from platform.openai.com
- **Model** (optional): Default `gpt-4o`
- **Base URL** (optional): For custom endpoints

### Supported Models
- `gpt-4o` (recommended)
- `gpt-4o-mini`
- `gpt-4-turbo`
- `gpt-3.5-turbo`

### Official Documentation
- [OpenAI API Docs](https://platform.openai.com/context/api-reference)
- [Chat Completions](https://platform.openai.com/context/api-reference/chat)
- [API Keys](https://platform.openai.com/api-keys)

### Key Features
- Fast response times
- Good value for cost
- Streaming support
- Function calling capability

### Headers Required
```
Authorization: Bearer <your-api-key>
```

### Pricing
- GPT-4o: $2.50/$10.00 per 1M tokens
- GPT-4o-mini: $0.15/$0.60 per 1M tokens
- GPT-4 Turbo: $10/$30 per 1M tokens
- GPT-3.5 Turbo: $0.50/$1.50 per 1M tokens

---

## Custom Endpoint

### Description
OpenAI-compatible custom LLM endpoint for self-hosted or alternative providers.

### Purpose
- Self-hosted LLM deployments
- Alternative LLM providers with OpenAI-compatible APIs
- Custom model deployments

### Configuration Required
- **API Key**: Authentication key for your service
- **Base URL**: Your LLM API endpoint
- **Model**: Model identifier

### Use Cases
- Self-hosted LLaMA, Mistral, or other open models
- Azure OpenAI Service
- Google Gemini (via compatible proxy)
- AWS Bedrock
- Local LLM deployments (Ollama, vLLM, etc.)

### Example Configuration

#### Azure OpenAI
```json
{
  "provider_type": "openai",
  "api_key": "your-azure-key",
  "base_url": "https://your-resource.openai.azure.com/openai/deployments/your-deployment",
  "model": "gpt-4"
}
```

#### Ollama Local
```json
{
  "provider_type": "openai",
  "api_key": "ollama",
  "base_url": "http://localhost:11434/v1",
  "model": "llama3.2"
}
```

#### Alibaba DashScope (Qwen)
```json
{
  "provider_type": "anthropic",
  "api_key": "your-dashscope-key",
  "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
  "model": "qwen-max"
}
```

---

## Local Resident Polish Runtime

### Description
Local GGUF polish models use an OpenAI-compatible localhost runtime instead of
loading GGUF files in the request path. The app checks the model file first,
then checks or starts the local runtime before the first polish request.

### Default Endpoint
```
http://127.0.0.1:8000/v1/chat/completions
```

### Settings Configuration

The desktop settings store a `local_polish_runtime` object:

```json
{
  "provider_type": "llama-server",
  "base_url": "http://127.0.0.1:8000/v1",
  "api_key": "",
  "server_command": "",
  "server_args_json": "",
  "ready_timeout_secs": 20
}
```

The Private AI > Polish page exposes presets for `llama-server`, `LM Studio`,
and `Ollama`, plus a custom endpoint. The check button validates the configured
runtime before users rely on local polish. When the selected polish model is
already downloaded, the same check prepares that model through the normal local
runtime path, so bundled, PATH-installed, or configured `llama-server`
processes can be started and verified. Without a downloaded selected model, the
check falls back to a lightweight `/v1/models` endpoint health check. The
selected model status separates file download state from runtime readiness; a
downloaded GGUF does not show as fully ready until the local runtime is also
reachable.

For the `llama-server` preset, leaving `server_command` empty makes AriaType
try to auto-detect a bundled `llama-server` resource first, then a
`llama-server` on `PATH`, before reporting the runtime unavailable. This code
path is ready for a packaged sidecar. Build scripts generate
`tauri.runtime.generated.conf.json` before packaging and add existing
`llama-server` sidecar binaries to Tauri resources without breaking builds when
those binaries are absent.

Recognized bundled sidecar locations are:

- macOS: `bin/apple-silicon/llama-server`, `bin/intel/llama-server`,
  `bin/universal/llama-server`, or `bin/macos/llama-server`
- Windows: `bin/windows/llama-server.exe` or `bin/windows/llama-server`
- Linux: `bin/linux/llama-server` or `bin/llama-server`

The repository still needs the actual sidecar binary resource to deliver a fully
bundled zero-configuration runtime.

Release and local package builds must provide that resource without committing a
large binary to source control. All packaging entry points call
`scripts/prepare-tauri-runtime-resources.mjs --require-runtime`, so installer
builds fail before Tauri packaging when the local polish sidecar is missing.
The preparation script accepts these environment variables before it writes
`tauri.runtime.generated.conf.json`:

| Variable | Purpose |
|----------|---------|
| `ARIATYPE_LLAMA_SERVER_MACOS_ARM64_PATH` | Path to a macOS arm64 `llama-server` binary copied to `bin/apple-silicon/llama-server` |
| `ARIATYPE_LLAMA_SERVER_MACOS_X64_PATH` | Path to a macOS x64 `llama-server` binary copied to `bin/intel/llama-server` |
| `ARIATYPE_LLAMA_SERVER_MACOS_PATH` | Path to a universal macOS `llama-server` binary copied to `bin/universal/llama-server` |
| `ARIATYPE_LLAMA_SERVER_WINDOWS_X64_PATH` | Path to a Windows x64 `llama-server.exe` binary copied to `bin/windows/llama-server.exe` |
| `ARIATYPE_LLAMA_SERVER_WINDOWS_PATH` | Fallback Windows `llama-server.exe` artifact path copied to `bin/windows/llama-server.exe` |
| `ARIATYPE_LLAMA_SERVER_LINUX_X64_PATH` | Path to a Linux x64 `llama-server` binary copied to `bin/linux/llama-server` |
| `ARIATYPE_LLAMA_SERVER_LINUX_PATH` | Fallback Linux `llama-server` artifact path copied to `bin/linux/llama-server` |
| `ARIATYPE_LLAMA_SERVER_<PLATFORM_OR_ARCH>_SHA256` | Optional checksum for the provided binary |
| `ARIATYPE_REQUIRE_LOCAL_POLISH_RUNTIME` | Set to `1` to fail the build when the sidecar artifact is missing |

The script is a no-op when no artifact path is configured and the runtime is not
required, so regular development builds stay lightweight. Local installer builds
must either already have the runtime in the recognized `src-tauri/bin/*`
location or provide the matching `ARIATYPE_LLAMA_SERVER_*_PATH` variable.
macOS discovery prefers the current architecture first, so a universal build can
ship both `bin/apple-silicon/llama-server` and `bin/intel/llama-server` without
Intel machines accidentally trying the Apple Silicon binary.

Windows packaging entry points now also try to prepare the pinned official
`llama-b<release>-bin-win-cpu-x64.zip` asset automatically when
`bin/windows/llama-server.exe` is missing and no `ARIATYPE_LLAMA_SERVER_WINDOWS*`
variable is configured. The asset is cached under `.tmp/llama-server-assets/`
and then copied through the same `prepare-llama-server-release-assets.mjs`
pipeline that the release workflow uses. The required-runtime gate still stays
on: the build only proceeds automatically if that download-and-prepare step
completes successfully.

The GitHub release workflow pins a llama.cpp release tag, downloads the official
macOS arm64, macOS x64, and Windows CPU x64 assets from
`ggml-org/llama.cpp`, extracts `llama-server`, and builds with
`ARIATYPE_REQUIRE_LOCAL_POLISH_RUNTIME=1`. A release build therefore fails
before packaging if the expected sidecar cannot be prepared. The official
runtime is not a single file: macOS assets also need sibling `.dylib` files,
Windows assets need sibling `.dll` files, and Linux assets may need sibling
`.so*` files. Release preparation copies those dependencies into the same Tauri
resource directory as `llama-server`, and generated Tauri resource config
includes them whenever the runtime executable is present. After packaging, the
workflow verifies the app bundle or Windows bundle still contains the expected
runtime resources and that the current-architecture runtime executable can
start far enough to answer `--help` before upload. The same verifier accepts an
explicit `--resource-root`, which allows release checks to verify a mounted DMG
app bundle instead of only the Tauri target directory.

### Environment Fallback

| Variable | Purpose | Default |
|----------|---------|---------|
| `ARIATYPE_LOCAL_POLISH_BASE_URL` | OpenAI-compatible local base URL | `http://127.0.0.1:8000/v1` |
| `ARIATYPE_LOCAL_POLISH_API_KEY` | Optional bearer token for the local runtime | unset |
| `ARIATYPE_LOCAL_POLISH_SERVER_COMMAND` | Optional command the app can spawn when no runtime is listening | unset |
| `ARIATYPE_LOCAL_POLISH_SERVER_ARGS_JSON` | JSON array of command arguments with placeholders | Provider-specific default args |
| `ARIATYPE_LOCAL_POLISH_READY_TIMEOUT_SECS` | Startup readiness timeout | `20` |

Environment variables are used before settings are loaded and remain useful for
advanced local runs. Once settings are loaded, the saved `local_polish_runtime`
configuration is the runtime source of truth.

Supported argument placeholders:

- `{model_path}`: absolute GGUF path in the AriaType models directory
- `{model_id}` / `{model_alias}`: selected polish model ID
- `{host}` / `{port}`: parsed host and port from the base URL
- `{base_url}`: configured base URL

For the `llama-server` provider, empty `server_args_json` uses the native
llama.cpp server CLI:

```bash
llama-server \
  --model "{model_path}" \
  --alias "{model_alias}" \
  --host "{host}" \
  --port "{port}"
```

For custom Python-based runtimes, set
`ARIATYPE_LOCAL_POLISH_SERVER_COMMAND=python3` and configure args explicitly for
llama-cpp-python:

```bash
python3 -m llama_cpp.server \
  --model "{model_path}" \
  --model_alias "{model_alias}" \
  --host "{host}" \
  --port "{port}"
```

The runtime verification script also supports a stronger optional server smoke
check when a real GGUF file is available:

```bash
node scripts/verify-tauri-runtime-resources.mjs --platform macos --smoke --server-model /path/to/model.gguf
```

If no runtime is listening and no spawn command is configured, local polish is
not considered ready, except that the `llama-server` preset first attempts
bundled-resource and PATH auto-detection. The app falls back to the original STT
text and reports a local polish server unavailable reason.

Before LLM polish runs, recording and retry flows execute deterministic
post-STT processing. This stage applies correction-learning mappings, explicit
glossary mappings, canonical casing hints, and conservative normalization for
whitespace and punctuation spacing. These corrections are part of the fast path
and do not require the local runtime.

When a polish request has a preview callback, the local runtime request uses
OpenAI-compatible streaming (`stream: true`). Incoming SSE chunks keep the
pill tooltip in a generic processing state instead of exposing raw streamed
model output. AriaType still inserts text only after the final polish result by
default.

An advanced `polish_stream_direct_typing_enabled` setting can type streamed
polish chunks directly into the target app during the recording flow. This mode
is disabled by default because already-typed chunks cannot be cleanly rolled
back if the provider later fails or changes direction. Retry still uses the
atomic final-result insertion path.

---

## Feature Comparison

| Feature | Anthropic | OpenAI | Custom |
|---------|-----------|--------|--------|
| Writing Quality | Excellent | Very Good | Varies |
| Response Speed | Fast | Very Fast | Varies |
| Context Window | 200K tokens | 128K tokens | Varies |
| Streaming | Yes | Yes | Varies |
| Thinking Mode | Yes | No | Varies |
| Pricing | Medium | Low-Medium | Varies |

---

## Thinking Mode

### Description
Extended reasoning capability where the model "thinks" before responding, improving output quality for complex polishing tasks.

### Availability
- **Anthropic Claude 3.5 Sonnet**: Supported
- **OpenAI GPT models**: Not supported
- **Custom**: Depends on provider

### Use Cases
- Complex document restructuring
- Technical writing polishing
- Content requiring careful reasoning

### Configuration
```json
{
  "enable_thinking": true
}
```

---

## Provider Selection Guide

### Choose Anthropic Claude when:
- Writing quality is the top priority
- You need nuanced style improvements
- Complex document restructuring is required
- Thinking mode for complex tasks is valuable

### Choose OpenAI GPT when:
- Speed is important
- Cost-effectiveness matters
- You need reliable, consistent output
- Integration simplicity is preferred

### Choose Custom Endpoint when:
- You have self-hosted LLM infrastructure
- You want to use alternative providers (Gemini, Qwen, etc.)
- Cost optimization through open models is needed
- Data privacy requires on-premise deployment

---

## API Request Format

### Anthropic Format
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 4096,
  "system": "You are a text polishing assistant...",
  "messages": [
    {"role": "user", "content": "Polish this text: ..."}
  ]
}
```

### OpenAI Format
```json
{
  "model": "gpt-4o",
  "max_tokens": 4096,
  "messages": [
    {"role": "system", "content": "You are a text polishing assistant..."},
    {"role": "user", "content": "Polish this text: ..."}
  ]
}
```

---

## Error Handling

### Common Errors

| Error Code | Provider | Meaning | Resolution |
|------------|----------|---------|------------|
| 401 | All | Invalid API key | Check API key configuration |
| 403 | Anthropic | Access denied | Verify model access and billing |
| 429 | All | Rate limited | Wait and retry, or upgrade plan |
| 500 | All | Server error | Retry with exponential backoff |

### Rate Limits
- **Anthropic**: Varies by tier, typically 60-1000 RPM
- **OpenAI**: Varies by tier, typically 500-10000 RPM
- **Custom**: Depends on deployment

---

## Best Practices

1. **Model Selection**: Start with recommended defaults, adjust based on quality/cost needs
2. **Token Management**: Monitor usage to avoid unexpected costs
3. **Error Handling**: Implement retry logic with exponential backoff
4. **Prompt Engineering**: Customize system prompts for specific use cases
5. **Testing**: Compare outputs across providers for your specific content type
