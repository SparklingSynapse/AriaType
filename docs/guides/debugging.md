# Debugging Guide

## When to Read This

- Read [`../../AGENTS.md`](../../AGENTS.md) for execution constraints, recovery protocol, and the default iteration loop
- Read [`../spec/logs.md`](../spec/logs.md) for logging requirements and structured field conventions
- Read this guide for concrete investigation workflow, log locations, and root-cause analysis steps
- Read [`../architecture/data-flow.md`](../architecture/data-flow.md) when debugging depends on pipeline stages, state transitions, or IPC boundaries

## 1. Log Investigation Workflow

When asked to "check and fix" or "investigate":

1. **Check latest logs first**:
   ```bash
   # macOS
   cat ~/Library/Logs/ariatype/ariatype.log.$(date +%Y-%m-%d-)* | tail -200
   
   # Or check for crash reports
   ls -la ~/Library/Logs/DiagnosticReports/ | grep -i ariatype
   ```

2. **Identify failure point** — Look for:
   - ERROR-level log entries
   - Stack traces or panic messages
   - Timestamps near the reported issue
   - Unexpected state transitions

3. **Check crash reports if logs incomplete**:
   ```bash
   head -200 ~/Library/Logs/DiagnosticReports/ariatype-*.ips
   ```

## 2. Log File Locations

| Platform | Path |
|----------|------|
| macOS | `~/Library/Logs/ariatype/` |
| Windows | `%LOCALAPPDATA%\ariatype\logs\` |

Log files are named `ariatype.log.YYYY-MM-DDTHH` (hourly rotation).

## 3. IPC Log Access

```typescript
// Get last N lines of log content
invoke("get_log_content", { lines: 100 });

// Open log folder in system file manager
invoke("open_log_folder");
```

## 4. Root Cause Analysis Protocol

1. Reproduce the issue deterministically
2. Collect direct evidence: logs, stack traces, diagnostics, failing assertions
3. Trace execution path across caller → callee → state → side effects → boundaries
4. Identify the precise failure point
5. State root cause as evidence-backed conclusion
6. Verify the fix removes root cause, not just symptom

## 5. Failure Recovery Pattern

After 3 consecutive failures:
1. STOP all further edits
2. REVERT to last known working state
3. DOCUMENT what was attempted and what failed
4. ESCALATE to user or consult Oracle with full failure context

## 6. Text Injection Debugging

- Layer 0 (keyboard simulation): For short text ≤200 chars, no newlines
- Layer 2 (clipboard paste): For long text >200 chars or multiline content
- Check `apps/desktop/src-tauri/docs/text_injection_fix.md` for known issues

## 7. Cloud Engine Debugging

- Auth errors (401/403): Check API key, app_id, base_url in settings
- Connection errors: Check network, WebSocket endpoint URL
- Timeout errors: Check provider status, rate limits
- Use mock credentials pattern for local testing
