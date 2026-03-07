# Polish Prompt Optimization for Small Models (<3B)

## Problem

With <3B parameter models (Qwen 0.8B-2B, LFM 1.2B-2.6B), the original prompts were:
1. **Too verbose**: Long instructions consume tokens and confuse small models
2. **English-only examples**: Misled models to translate Chinese input to English
3. **Complex formatting**: Agent template had too many markdown rules

Example failure:
- Input (Chinese): "系统性的梳理我们当前产品中的功能..."
- Output (English): "## Task\nSystematically document..." ❌

## Optimization Strategy

### 1. Simplify Language
- **Before**: "You are a text polishing assistant. Your job is MINIMAL editing."
- **After**: "Polish text minimally. Keep the SAME language as input."

### 2. Reduce Token Count
- Removed verbose explanations
- Shortened rule descriptions
- Kept only essential instructions

### 3. Add Bilingual Examples
Every template now includes both English and Chinese examples:
```
- "Um, I think this is good" → "I think this is good"
- "嗯，我觉得这个挺好的" → "我觉得这个挺好的"
```

### 4. Emphasize Language Preservation
Moved "SAME LANGUAGE" to the first rule in every prompt:
```
RULES:
1. SAME LANGUAGE: Chinese → Chinese, English → English
```

## Changes by Template

### Default Polish Prompt
- **Token reduction**: ~180 → ~90 tokens
- **Key change**: Added Chinese examples, simplified rules
- **Focus**: Minimal editing, language preservation

### Filler Template
- **Token reduction**: ~150 → ~80 tokens
- **Key change**: Added Chinese filler words (嗯, 那个, 就是说)
- **Focus**: Remove fillers only, no rewriting

### Formal Template
- **Token reduction**: ~120 → ~90 tokens
- **Key change**: Added Chinese formal conversion example
- **Focus**: Style conversion while preserving language

### Concise Template
- **Token reduction**: ~130 → ~85 tokens
- **Key change**: Added Chinese conciseness example
- **Focus**: Shorten without losing meaning

### Agent Template (Most Critical)
- **Token reduction**: ~250 → ~110 tokens (56% reduction!)
- **Key changes**:
  - Removed complex formatting guidelines
  - Added Chinese markdown example
  - Simplified to basic structure (## headers, - lists)
  - Removed "Requirements" section complexity
- **Focus**: Simple markdown formatting, language preservation

## Performance Benefits

1. **Faster inference**: Fewer prompt tokens = faster generation
2. **Better accuracy**: Simpler instructions = better following
3. **Language preservation**: Bilingual examples prevent translation
4. **Lower memory**: Shorter prompts fit better in small model context

## Testing

All templates tested with:
- English input → English output ✓
- Chinese input → Chinese output ✓
- Mixed content handling ✓
- No-change scenarios ✓

## Recommendations

For <3B models:
1. Keep prompts under 100 tokens when possible
2. Always include examples in target languages
3. Use simple, direct language
4. Avoid complex multi-step instructions
5. Emphasize critical rules (like language preservation) multiple times
