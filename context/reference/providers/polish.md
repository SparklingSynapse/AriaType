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