# Text Injection Fix for Long Polish Output

## Problem

When using the polish feature with long outputs (>200 characters) or multiline markdown text, the injected text was corrupted or truncated.

Example from logs:
- **Expected output**: 423 characters of formatted markdown
- **Actual output**: "ly document and orgare clarity and compling" (corrupted/truncated)

## Root Cause

The text injector was using `enigo.text()` (Layer 0) for all text lengths, which simulates keyboard input character by character. For long text or text with special characters (markdown formatting, newlines), this fast keyboard simulation can cause:

1. **Character loss**: Some applications can't process rapid keyboard events fast enough
2. **Buffer overflow**: Input buffers may drop characters when overwhelmed
3. **Special character issues**: Markdown symbols (`#`, `-`, newlines) can trigger unexpected behavior

## Solution

Modified the injection strategy in `text_injector/macos.rs`:

```rust
// For long text (>200 chars) or text with newlines, use clipboard paste directly
if text.len() > 200 || text.contains('\n') {
    info!("inject: using L2 (clipboard paste) for long/multiline text");
    let ok = try_clipboard_paste(text);
    return;
}
```

### Injection Strategy

- **Layer 0 (enigo keyboard simulation)**: Used for short text (<= 200 chars, no newlines)
  - Fast and doesn't touch clipboard
  - Best for simple, short transcriptions

- **Layer 2 (clipboard paste)**: Used for long text or multiline text
  - More reliable for complex content
  - Saves and restores clipboard content automatically
  - Uses `osascript` to simulate Cmd+V

## Testing

To verify the fix:

1. Enable polish feature with "agent" template
2. Speak a complex request that generates >200 chars of markdown output
3. Verify the complete formatted output is injected correctly

Example test input:
```
系统性的梳理我们当前产品中的功能，然后用这些汇总信息重新整理我们的 README 文件。
```

Expected output: Complete markdown with headers, lists, and formatting intact.

## Future Improvements

1. Consider adding configurable threshold for Layer 0 vs Layer 2
2. Implement chunked keyboard input with delays as Layer 1 fallback
3. Add retry logic for failed injections
4. Implement similar fix for Windows platform (currently TODO)
