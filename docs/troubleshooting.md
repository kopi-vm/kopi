# Kopi Troubleshooting Guide

This guide helps you diagnose and fix common issues with kopi. For automated diagnostics, run `kopi doctor`.

## Automated Diagnostics

Before manually troubleshooting, run the doctor command for comprehensive diagnostics:

```bash
kopi doctor                    # Run all diagnostic checks
kopi doctor --verbose         # Show detailed information
kopi doctor --check network   # Check specific category
kopi doctor --json           # Output in JSON format
```

The doctor command checks:
- Installation integrity
- Shell configuration
- JDK installations
- File permissions
- Network connectivity
- Cache status

## Enhanced Error Messages

Kopi provides comprehensive error messages with helpful suggestions when something goes wrong:

```bash
# Example: Version not found
$ kopi install 999
Error: JDK version 'temurin 999' is not available

Details: Version lookup failed: temurin 999 not found

Suggestion: Run 'kopi cache search' to see available versions or 'kopi cache refresh' to update the list.
```

## Common Issues and Solutions

### 1. Version Not Available
```bash
Error: JDK version 'X' is not available
```
**Solution:** 
- Run `kopi cache refresh` to update the metadata
- Use `kopi cache search <version>` to find available versions
- Check if you're using the correct distribution name

### 2. Already Installed
```bash
Error: temurin 21 is already installed
```
**Solution:** Use `--force` flag to reinstall:
```bash
kopi install 21 --force
```

### 3. Network Issues
```bash
Error: Failed to download JDK
```
**Solution:**
- Check your internet connection
- If behind a corporate proxy, set proxy environment variables (see HTTP Proxy Configuration in reference manual)
- Use `--timeout` to increase timeout for slow connections
- Try again later if rate limited

### 4. Permission Denied
```bash
Error: Permission denied: /path/to/directory
```
**Solution:**
- On Unix/macOS: Use `sudo` or check file permissions
- On Windows: Run as Administrator
- Ensure you have write access to `~/.kopi` directory

### 5. Disk Space
```bash
Error: Insufficient disk space
```
**Solution:**
- Free up disk space (JDK installations require 300-500MB)
- Configure minimum space in `~/.kopi/config.toml`
- Remove unused JDK versions with `kopi uninstall`

### 6. Checksum Mismatch
```bash
Error: Checksum verification failed
```
**Solution:**
- Try downloading again (file may be corrupted)
- If problem persists, report issue as it may be a source problem

### 7. Cache Not Found
```bash
Error: Cache not found
```
**Solution:** Run `kopi cache refresh` to fetch the latest JDK metadata

### 8. Uninstall Issues

#### Multiple JDKs Match:
```bash
Error: Multiple JDKs match version '21'
```
**Solution:** Use exact specification:
```bash
kopi uninstall temurin@21.0.5+11   # Specify exact version
kopi uninstall corretto@21          # Specify distribution
```

#### Files in Use (Windows):
```bash
Error: Files may be held by antivirus software
```
**Solution:**
- Close any running Java applications
- Temporarily disable real-time antivirus protection
- Add kopi directory to antivirus exclusions
- Use `--force` flag to attempt forced removal

#### Permission Errors:
```bash
Error: Permission denied removing JDK
```
**Solution:**
- Run as Administrator (Windows) or with sudo (Unix)
- Check file permissions in `~/.kopi/jdks/`
- Ensure no files are read-only or locked

#### Partial Removal Cleanup:
```bash
Error: Partial removal detected
```
**Solution:**
- Run `kopi uninstall --cleanup` to clean up partial removals
- Use `kopi uninstall --cleanup --force` to attempt stubborn file removal
- Check for orphaned temporary directories and metadata files

#### Orphaned Symlinks (Unix):
```bash
Warning: Orphaned symlinks found
```
**Solution:**
- Automatic cleanup during uninstall
- Manual cleanup: `find ~/.kopi -type l ! -exec test -e {} \; -delete`

## Shim-Specific Issues

### 1. Shim Not Working
```bash
Error: Tool 'java' not found in JDK
```
**Solution:**
- Ensure `~/.kopi/shims` is in your PATH
- Run `kopi shim verify` to check shim integrity
- Recreate the shim: `kopi shim add java --force`

### 2. Version Not Switching
```bash
# Wrong Java version despite .kopi-version file
```
**Solution:**
- Check version file location: must be in current or parent directory
- Verify version format: `temurin@21` or just `21`
- Check environment variable: `KOPI_JAVA_VERSION` overrides files
- Enable debug logging: `RUST_LOG=kopi::shim=debug java -version`

### 3. Performance Issues
```bash
# Slow shim execution
```
**Solution:**
- Run benchmarks: `cargo bench --bench shim_bench`
- Check for antivirus interference on Windows
- Ensure shims are built with release profile
- Verify no network delays in version resolution

### 4. Security Validation Errors
```bash
Error: Security error: Path contains directory traversal
```
**Solution:**
- Check for suspicious patterns in version files
- Ensure no malformed symlinks in kopi directories
- Run `kopi shim verify --fix` to repair issues

## Platform-Specific Issues

### Windows

#### PowerShell Execution Policy
```bash
Error: Scripts cannot execute in PowerShell
```
**Solution:**
```powershell
# Check current policy
Get-ExecutionPolicy

# Set policy for current user
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

#### Path Length Limitations
```bash
Error: Path too long
```
**Solution:**
- Enable long path support (Windows 10+):
  ```cmd
  reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1
  ```
- Restart computer

### macOS

#### Gatekeeper Issues
```bash
Error: "kopi" cannot be opened because the developer cannot be verified
```
**Solution:**
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine ~/.kopi/bin/kopi
```

#### Shell Configuration
- Ensure `.bash_profile` sources `.bashrc` if using bash
- For zsh (default on macOS 10.15+), use `~/.zshrc`

### Linux

#### SELinux Contexts
```bash
Error: Permission denied (even with correct file permissions)
```
**Solution:**
```bash
# Check SELinux status
sestatus

# Set correct context
restorecon -Rv ~/.kopi
```

## Shell Integration Problems

### PATH Not Updated
**Symptoms:**
- `java -version` shows wrong version
- `which java` doesn't point to kopi shims
- `kopi` command not found

**Solutions:**
1. Verify shims in PATH:
   ```bash
   echo $PATH | grep kopi/shims
   ```

2. Ensure correct PATH order:
   ```bash
   export PATH="$HOME/.kopi/shims:$PATH"
   ```

3. Check if kopi is installed:
   ```bash
   which kopi
   ```

### Version Not Detected
**Symptoms:**
- Kopi doesn't detect project version files
- Wrong version is used despite having `.kopi-version` or `.java-version`

**Solutions:**
1. Check file permissions:
   ```bash
   ls -la .kopi-version .java-version
   ```

2. Verify version format:
   ```bash
   cat .kopi-version  # Should be like: temurin@21
   cat .java-version  # Should be like: 21
   ```

3. Check version resolution:
   ```bash
   kopi current  # Shows which version is currently active
   kopi env  # Outputs environment variables for the current version
   ```

### Shell Detection Issues
**Symptoms:**
- `kopi env` fails to detect shell type
- Wrong shell syntax is generated

**Solutions:**
1. Explicitly specify shell:
   ```bash
   eval "$(kopi env --shell bash)"    # For bash
   kopi env --shell fish | source      # For fish
   kopi env --shell powershell | Invoke-Expression  # For PowerShell
   ```

2. Check your current shell:
   ```bash
   echo $SHELL
   echo $0
   ```

## Getting Help

If you encounter issues not covered here:

1. Run comprehensive diagnostics:
   ```bash
   kopi doctor --verbose > kopi-diagnostics.txt
   ```

2. Run commands with verbose logging:
   ```bash
   kopi install 21 -vv
   ```

3. Check the GitHub issues: https://github.com/anthropics/claude-code/issues

4. For feedback or bug reports, please report the issue at:
   https://github.com/anthropics/claude-code/issues

   Include:
   - Output from `kopi doctor`
   - Your operating system and version
   - Steps to reproduce the problem
   - Any error messages