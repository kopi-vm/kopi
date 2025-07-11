# Kopi Uninstall Command Design

## Overview

The `kopi uninstall` command removes installed JDK distributions from the local system. This command is essential for disk space management and keeping the JDK installation directory clean.

## Command Syntax

### Basic Usage
```bash
# Uninstall specific version
kopi uninstall 21
kopi uninstall temurin@21
kopi uninstall temurin@21.0.5+11
kopi uninstall jre@corretto@17

# Uninstall all versions of a distribution
kopi uninstall corretto --all

# Uninstall with options
kopi uninstall 21 --force
kopi uninstall 21 --dry-run
```

### Command Options
| Option | Short | Description |
|--------|-------|-------------|
| `--force` | `-f` | Force uninstall even if JDK is in use |
| `--dry-run` | | Show what would be removed without actually removing |
| `--all` | | Remove all versions of specified distribution |

## Functional Requirements

### 1. JDK Selection Logic

The uninstall command should follow the same selection logic as the install command for consistency:

```bash
# If multiple JDKs match "21", show selection prompt
kopi uninstall 21
> Select JDK to uninstall:
  1) temurin@21.0.5+11 (Default)
  2) corretto@21.0.5.11.1
  3) zulu@21.0.5+11

# Direct specification bypasses selection
kopi uninstall corretto@21.0.5.11.1
```

### 2. Safety Checks

#### Currently Active JDK
- Check if the JDK to be uninstalled is currently active (global or local)
- Require `--force` flag to uninstall active JDK
- Show clear warning message

```bash
kopi uninstall temurin@21
Error: Cannot uninstall temurin@21.0.5+11 - it is currently active
       Use --force to override this check
```

### 3. Removal Process

#### Components to Remove
- **JDK Installation Directory**: `~/.kopi/jdks/<distribution>-<version>/`

#### Shim Handling
- Shims should NOT be removed during uninstall
- Shims automatically redirect to next available JDK
- Only remove shims via dedicated command or when no JDKs remain

#### Metadata Updates
- Update cached metadata to reflect removal
- Do NOT remove distribution from cache (may want to reinstall later)

### 4. Batch Operations

#### Uninstall All Versions
```bash
# Remove all Corretto installations
kopi uninstall corretto --all
> This will remove:
  - corretto@8.432.06.1
  - corretto@11.0.25.9.1
  - corretto@17.0.13.11.1
  - corretto@21.0.5.11.1
  Total: 1.2 GB will be freed
Continue? [y/N]
```

#### Pattern-based Uninstall (Future Enhancement)
```bash
# Remove all non-LTS versions
kopi uninstall --non-lts

# Remove all JRE installations
kopi uninstall jre --all
```

## Implementation Details

### 1. Directory Structure
```
~/.kopi/
├── jdks/
│   ├── temurin-21.0.5+11/    # To be removed
│   └── corretto-21.0.5.11.1/
└── bin/
    └── java                   # Shim - NOT removed
```

### 2. Uninstall Flow
```
1. Parse command arguments
2. Resolve JDK(s) to uninstall
3. Perform safety checks
   - Check if active
   - Calculate disk space to be freed
4. Show confirmation (unless --force)
5. Remove JDK directory
6. Update metadata
7. Show completion message with freed space
```

### 3. Error Handling

| Error Type | Exit Code | Description |
|------------|-----------|-------------|
| JDK not found | 4 | Specified JDK is not installed |
| Active JDK | 10 | Attempting to uninstall active JDK without --force |
| Permission denied | 13 | Insufficient permissions to remove files |
| Partial removal | 14 | Some files could not be removed |

### 4. Disk Space Calculation
```rust
// Calculate total space to be freed
let total_size = calculate_directory_size(&jdk_path)?;

// Display in human-readable format
println!("This will free {}", format_size(total_size));
```

## User Experience Considerations

### 1. Interactive Confirmation
- Always ask for confirmation unless `--force` is used
- Show exactly what will be removed
- Display disk space to be freed

### 2. Progress Indication
- Show progress for large removals
- Use spinner for directory scanning
- Clear completion message

### 3. Helpful Messages
```bash
# After successful uninstall
✓ Successfully uninstalled corretto@21.0.5.11.1
  Freed 312 MB of disk space

# If it was the last JDK
Warning: No JDKs remaining. Run 'kopi install' to install a JDK.

# If it was the active JDK
Warning: Removed active JDK. Run 'kopi use' to select another JDK.
```

## Integration with Other Commands

### 1. List Command Enhancement
```bash
kopi list
# Show installed size for each JDK
temurin@21.0.5+11 (312 MB)
corretto@21.0.5.11.1 (298 MB) ← current
```

### 2. Prune Command (Future)
```bash
# Remove all unused JDKs
kopi prune
# Remove JDKs not used in last N days
kopi prune --older-than 90d
```

### 3. Doctor Command
- Report orphaned directories
- Check for incomplete uninstalls
- Suggest cleanup actions

## Security Considerations

### 1. Permission Checks
- Verify user has permission to remove directories
- Handle permission errors gracefully
- Never use sudo/elevation internally

### 2. Path Validation
- Ensure removal paths are within kopi directory
- Prevent directory traversal attacks
- Validate JDK directory structure before removal

### 3. Atomic Operations
- Use rename-to-temp then remove pattern
- Ensure rollback capability on failure
- Prevent partial removals

## Platform-Specific Considerations

### Unix/Linux/macOS
- Use standard file operations
- Handle symbolic links properly
- Respect file permissions

### Windows
- Handle files in use (may need reboot)
- Consider antivirus interference

## Summary

The `kopi uninstall` command provides a safe and user-friendly way to remove JDK installations. Key features include:

1. **Safety First**: Prevents accidental removal of active JDKs
2. **Clear Feedback**: Shows what will be removed and disk space impact
3. **Flexible Selection**: Supports specific versions or bulk removal
4. **Clean Removal**: Removes JDK installations completely
5. **Smart Integration**: Works seamlessly with other kopi commands

This design ensures users can confidently manage their JDK installations while preventing common mistakes like removing the currently active JDK.