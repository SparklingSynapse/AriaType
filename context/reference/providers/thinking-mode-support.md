# Thinking Mode Support by Provider

Whether reasoning/thinking mode can be disabled for major LLM providers and their models. All information below is verified against official provider documentation.

> Last updated: 2026-04-10

## Quick Reference

| Provider | Model | Thinking Default | Can Disable? | API Parameter | Official Source |
|----------|-------|----------------|-------------|-------------|----------------|
| **OpenAI** | o1, o1-mini, o1-pro | Always on | ❌ No | `reasoning_effort: low/medium/high` (reduce only) | [platform.openai.com/context/guides/reasoning](https://platform.openai.com/context/guides/reasoning) |
| | o3, o3-mini | Always on | ❌ No | `reasoning_effort: low/medium/high` | |
| | GPT-5.4, GPT-5.3-codex, etc. | Varies | ✅ Yes | `reasoning_effort: none/low/medium/high/xhigh` | |
| | GPT-4o | No thinking | N/A | — | |
| **Anthropic** | Claude Opus 4.6, Sonnet 4.6 | Adaptive (on) | ✅ Yes | `thinking: {"type": "disabled"}` | [docs.anthropic.com/en/context/build-with-claude/extended-thinking](https://docs.anthropic.com/en/context/build-with-claude/extended-thinking) |
| | Claude Opus 4.5, Sonnet 4.5, Haiku | Off by default | ✅ Yes | `thinking: {"type": "disabled"}` | |
| | Claude Opus 4, Sonnet 4 | Off by default | ✅ Yes | `thinking: {"type": "disabled"}` | |
| | Claude Mythos Preview | Adaptive (on) | ❌ **Not supported** | — | |
| **Google** | Gemini 2.5 Pro | Always on | ❌ No | Cannot disable; min budget 128 tokens | [ai.google.dev/gemini-api/context/thinking](https://ai.google.dev/gemini-api/context/thinking) |
| | Gemini 2.5 Flash | Dynamic (on) | ✅ Yes | `thinkingConfig.thinkingBudget: 0` | |
| | Gemini 2.5 Flash Lite | No thinking | N/A | — | |
| | Gemini 3.1 Pro | Always on | ❌ No | `thinkingLevel: "minimal"` not supported | |
| | Gemini 3.1 Flash, Flash-Lite | Dynamic (on) | ✅ Yes | `thinkingLevel: "low"` | |
| **DeepSeek** | deepseek-reasoner (R1) | Always on | ❌ No | Dedicated endpoint | [api-docs.deepseek.com/guides/thinking_mode](https://api-docs.deepseek.com/guides/thinking_mode) |
| | deepseek-chat (V3.2) | Off by default | ✅ Yes | `extra_body={"thinking": {"type": "enabled"}}` to opt-in | |
| **Zhipu (智谱)** | GLM-5, GLM-4.7 | Always on | ❌ **No** | `thinking: {"type": "disabled"}` accepted but **forced** by model design | [docs.bigmodel.cn/cn/guide/capabilities/thinking](https://docs.bigmodel.cn/cn/guide/capabilities/thinking) |
| | GLM-4.5, GLM-4.6 | Dynamic (on) | ✅ Yes | `thinking: {"type": "disabled"}` | |
| **Qwen (通义千问)** | QwQ-Plus, QwQ-32B | Always on | ❌ No | Thinking-only model | [www.alibabacloud.com/help/en/model-studio/qwq](https://www.alibabacloud.com/help/en/model-studio/qwq) |
| | Qwen3 (hybrid variants) | Mixed* | ✅ Yes | `extra_body={"enable_thinking": false}` | |
| | Qwen3 (thinking-only variants) | Always on | ❌ No | Model name ends with `-thinking` | |
| | Qwen2.5, Qwen2 | No thinking | N/A | — | |
| **ByteDance (豆包)** | Doubao-1.5-pro, 1.5-lite | No thinking | N/A | — | [www.volcengine.com/context/6492/2165101](https://www.volcengine.com/context/6492/2165101) |
| | Doubao-1.5-thinking-pro | Always on | ✅ Yes | `thinking: {"type": "disabled"}` | |
| | Doubao-Seed-1.6, 1.6-flash | Dynamic (auto) | ✅ Yes | `thinking: {"type": "disabled"}` | |
| | Doubao-Seed-1.6-thinking | Always on | ✅ Yes | `thinking: {"type": "disabled"}` | |
| **MiniMax** | M2.7, M2.7-highspeed | Always on | ❌ No | `thinking: {"type": "disabled"}` is accepted but **ignored** | [platform.minimax.io/context/guides/text-m2-function-call](https://platform.minimax.io/context/guides/text-m2-function-call) |
| | M2.5, M2.5-highspeed | Always on | ❌ No | Same — cannot disable | |

> * Qwen3 hybrid defaults vary: Qwen3.5 series on by default; Qwen3, Qwen3-VL, Qwen3-Omni-Flash off by default. All hybrid models can toggle.

## API Parameter Details

### OpenAI

Official docs: https://platform.openai.com/context/guides/reasoning

```python
# o-series: can only reduce effort, cannot fully disable
response = client.chat.completions.create(
    model="o3",
    messages=[...],
    reasoning_effort="low"  # low / medium / high only
)

# GPT-5 series: can also set to "none" to disable
response = client.chat.completions.create(
    model="gpt-5.4",
    messages=[...],
    reasoning_effort="none"  # none / minimal / low / medium / high / xhigh
)
```

### Anthropic

Official docs: https://docs.anthropic.com/en/context/build-with-claude/extended-thinking

```python
# Disable thinking (all supported models except Mythos Preview)
response = client.messages.create(
    model="claude-sonnet-4-6",
    thinking={"type": "disabled"},
    max_tokens=1024,
    messages=[...]
)

# Adaptive thinking (Opus 4.6, Sonnet 4.6 — recommended)
thinking={"type": "adaptive", "effort": "high"}  # low / medium / high

# Manual mode (Opus 4.5, Sonnet 4.5, Haiku — deprecated on 4.6)
thinking={"type": "enabled", "budget_tokens": 10000}
```

> **Note:** Claude Mythos Preview does NOT support `thinking: {"type": "disabled"}`.

### Google Gemini

Official docs: https://ai.google.dev/gemini-api/context/thinking

```python
# Gemini 2.5 series — use thinkingBudget
from google.genai import types
config=types.GenerateContentConfig(
    thinking_config=types.ThinkingConfig(thinking_budget=0)  # disable
    # thinking_budget=-1  dynamic (default)
    # thinking_budget=1024  fixed budget
)
```

| Model | Can Disable? | Parameter | Range |
|-------|-------------|-----------|-------|
| Gemini 2.5 Pro | ❌ No | `thinkingBudget` | 128–32768 (cannot set 0) |
| Gemini 2.5 Flash | ✅ Yes | `thinkingBudget: 0` | 0–24576 |
| Gemini 2.5 Flash Lite | N/A | No thinking | — |
| Gemini 3.1 Pro | ❌ No | `thinkingLevel: "minimal"` not supported | — |
| Gemini 3.1 Flash/Flash-Lite | ✅ Yes | `thinkingLevel: "low"` | minimal/low/medium/high |

### DeepSeek

Official docs: https://api-docs.deepseek.com/guides/thinking_mode

```python
# R1 — always thinks, dedicated reasoning endpoint
response = client.chat.completions.create(
    model="deepseek-reasoner",
    messages=[...]
)

# V3.2 — thinking off by default; enable with extra_body
response = client.chat.completions.create(
    model="deepseek-chat",
    messages=[...],
    extra_body={"thinking": {"type": "enabled"}}
)
```

### Zhipu (智谱)

Official docs: https://docs.bigmodel.cn/cn/guide/capabilities/thinking

```python
# GLM-4.5, GLM-4.6 — can disable
response = client.chat.completions.create(
    model="glm-4.6",
    messages=[...],
    extra_body={"thinking": {"type": "disabled"}}
)

# GLM-5, GLM-4.7 — cannot disable
# Official docs: "glm-5 glm-4.7 glm-4.5v 为强制思考" (forced thinking)
# sending {"type": "disabled"} is accepted but model still thinks
```

> **GLM-5 and GLM-4.7**: Official documentation explicitly states these are "forced deep thinking" (`强制思考`). The `thinking.type: disabled` parameter is listed in the API schema but the models always produce thinking content. This mirrors MiniMax's behavior.

### Qwen (通义千问)

Official docs: https://www.alibabacloud.com/help/en/model-studio/qwq

```python
# Hybrid models — can disable
response = client.chat.completions.create(
    model="qwen3-32b",
    messages=[...],
    extra_body={"enable_thinking": False}
)

# With budget (DashScope / vLLM / SGLang deployments)
extra_body={"enable_thinking": True, "thinking_budget": 500}
```

| Model | Thinking Default | Can Disable? |
|-------|----------------|-------------|
| QwQ-Plus, QwQ-32B | Always on | ❌ No (thinking-only) |
| Qwen3.5-plus, qwen3.5-flash | On by default | ✅ Yes |
| Qwen3, Qwen3-VL, Qwen3-Omni-Flash | Off by default | ✅ Yes |
| Qwen3-Omni-Turbo | No thinking | N/A |
| Model names ending in `-thinking` | Always on | ❌ No (thinking-only) |

### ByteDance (豆包)

Official docs: https://www.volcengine.com/context/6492/2165101

```python
# OpenAI-compatible API — disable thinking
response = client.chat.completions.create(
    model="doubao-seed-1-6-251015",
    messages=[...],
    extra_body={"thinking": {"type": "disabled"}}
)
# Values: "enabled" / "disabled" / "auto"
```

```python
# LAS/Daft API — thinking_type parameter
ArkLLMThinkingVision,
construct_args={
    "model": "doubao-seed-1.6",
    "thinking_type": "disabled",  # enabled / disabled / auto
}
```

### MiniMax

Official docs: https://platform.minimax.io/context/guides/text-m2-function-call

```python
# WARNING: parameter is accepted but IGNORED
# MiniMax models always return thinking content regardless
response = client.messages.create(
    model="MiniMax-M2.7-highspeed",
    thinking={"type": "disabled"},  # accepted but ineffective
    max_tokens=1024,
    messages=[...]
)
```
