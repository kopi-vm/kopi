---
name: win
description: Investigate Windows test failures by analyzing error logs
parameters:
  - name: error_log
    description: The error log text from the failed Windows test
    required: true
---

# Windows Test Failure Investigation

I'll analyze the Windows test failure based on the provided error log.

## Analysis Steps:

1. **Parse Error Messages**: Identify specific error types and patterns in the log
2. **Check Windows-Specific Issues**:
   - Path separator issues (`\` vs `/`)
   - File permission problems
   - Symbolic link/junction limitations
   - Case sensitivity differences
   - Line ending issues (CRLF vs LF)
   - Long path limitations (260 characters)
3. **Identify Root Cause**: Determine the primary failure reason
4. **Suggest Solutions**: Provide actionable fixes

## Error Log Analysis:

```
{{error_log}}
```

Let me analyze this error log and identify the Windows-specific issues...

<task>
# First, I'll examine the error log for common Windows test failure patterns

- Look for path-related errors (incorrect separators, absolute paths)
- Check for permission denied errors
- Identify any symbolic link or junction failures
- Search for encoding or line ending issues
- Look for platform-specific API differences

# Then I'll:
1. Identify the specific test(s) that failed
2. Determine if it's a Windows-specific issue or a general bug
3. Suggest concrete fixes or workarounds
4. Recommend any necessary platform-specific code changes
</task>