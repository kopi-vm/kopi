# Kopi Uninstall Command Design

## Implementation Status

**Current Status**: Partially implemented (core functionality complete, CLI integration pending)

### Implemented Components
- ✅ Core uninstall functionality (`src/uninstall/mod.rs`)
- ✅ Batch uninstall support (`src/uninstall/batch.rs`)
- ✅ Safety checks framework (`src/uninstall/safety.rs` - stubs only)
- ✅ JDK selection logic (`src/uninstall/selection.rs`)
- ✅ Version pattern matching (flexible matching for extended formats)
- ✅ Atomic removal with rollback capability
- ✅ Progress indicators for large removals
- ✅ Disk space calculation and reporting
- ✅ Integration tests

### Pending Implementation
- ❌ CLI integration (command not added to `main.rs`)
- ❌ `--force` flag handling
- ❌ Active JDK detection (safety check stubs always return false)
- ❌ Running process detection
- ❌ `--all` flag in CLI

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
| Option | Short | Description | Status |
|--------|-------|-------------|--------|
| `--force` | `-f` | Force uninstall even if JDK is in use | ❌ Not implemented |
| `--dry-run` | | Show what would be removed without actually removing | ✅ Implemented |
| `--all` | | Remove all versions of specified distribution | ✅ Logic implemented, CLI pending |

## Functional Requirements

### 1. JDK Selection Logic

The uninstall command requires exact JDK specification when multiple JDKs match a pattern:

```bash
# If multiple JDKs match "21", show error with clear instructions
kopi uninstall 21
Error: Multiple JDKs match the pattern '21'

Found the following JDKs:
  - temurin@21.0.5+11
  - corretto@21.0.5.11.1
  - zulu@21.0.5+11

Please specify exactly one JDK to uninstall using the full version:
  kopi uninstall <distribution>@<full-version>

Example:
  kopi uninstall temurin@21.0.5+11

# Direct specification works immediately
kopi uninstall corretto@21.0.5.11.1
```

#### Version Pattern Matching

The uninstall command supports flexible version pattern matching to handle distributions with extended version formats:

```bash
# Pattern matching for standard 3-component versions
kopi uninstall temurin@21.0.5    # Matches temurin@21.0.5+11

# Pattern matching for extended formats (4+ components)
kopi uninstall corretto@21       # Matches corretto@21.0.5.11.1
kopi uninstall corretto@21.0     # Matches corretto@21.0.5.11.1
kopi uninstall corretto@21.0.5   # Matches corretto@21.0.5.11.1
kopi uninstall corretto@21.0.5.11 # Matches corretto@21.0.5.11.1

# Pattern matching for 6-component versions (e.g., Dragonwell)
kopi uninstall dragonwell@21.0.7.0.7 # Matches dragonwell@21.0.7.0.7.6
```

This flexible matching ensures that users can uninstall JDKs regardless of the version format used by different distributions.

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

**Implementation Note**: Safety check functions (`is_active_global_jdk` and `is_active_local_jdk`) are currently stubs that always return `false`. These will be implemented when the `global` and `local` commands are available.

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

**Implementation Note**: The removal process uses an atomic rename-then-delete pattern:
1. Rename JDK directory to `.{jdk-name}.removing`
2. Delete the renamed directory
3. Rollback on failure by renaming back

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

**Implementation Status**: The batch uninstall logic is fully implemented in `BatchUninstaller`, including:
- Multi-progress bars for visual feedback
- Transaction-like behavior (report all successes/failures)
- Confirmation prompts (unless `--force`)
- Per-JDK safety checks

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

### Current Module Structure
```
src/uninstall/
├── mod.rs         # Main UninstallHandler implementation
├── batch.rs       # BatchUninstaller for --all operations
├── safety.rs      # Safety checks (stubs for now)
└── selection.rs   # JDK selection utilities
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

| Error Type | Exit Code | Description | Implementation Status |
|------------|-----------|-------------|----------------------|
| JDK not found | 4 | Specified JDK is not installed | ✅ Implemented |
| Active JDK | 10 | Attempting to uninstall active JDK without --force | ❌ Safety checks are stubs |
| Permission denied | 13 | Insufficient permissions to remove files | ✅ Implemented |
| Partial removal | 14 | Some files could not be removed | ✅ Handled via rollback |

### 4. Disk Space Calculation
```rust
// Calculate total space to be freed
let total_size = calculate_directory_size(&jdk_path)?;

// Display in human-readable format
println!("This will free {}", format_size(total_size));
```

**Implementation Note**: Disk space calculation is implemented using `JdkRepository::get_jdk_size()` which recursively calculates directory size. The `format_size()` function provides human-readable output (B, KB, MB, GB, TB).

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

### 4. Design Decision: No Interactive Selection
When multiple JDKs match a pattern, the command displays an error with clear instructions rather than prompting for interactive selection. This ensures:
- Predictable behavior in automated scripts
- Clear, explicit JDK removal operations
- Prevention of accidental removals
- Consistent behavior across all environments

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

## Next Steps for Full Implementation

1. **CLI Integration**: Add `Uninstall` variant to the `Commands` enum in `main.rs`
2. **Force Flag**: Implement `--force` flag to bypass safety checks
3. **Active JDK Detection**: Replace stub functions when `global` and `local` commands are ready
4. **Process Detection**: Implement checking for running Java processes
5. **List Command Enhancement**: Show installed size in `kopi list` output

## Summary

The `kopi uninstall` command core functionality is complete and well-tested. The implementation provides:

1. **Safety Framework**: Structure for preventing accidental removal (awaiting active JDK detection)
2. **Clear Feedback**: Shows what will be removed and disk space freed
3. **Flexible Selection**: Pattern matching supports all distribution version formats
4. **Atomic Removal**: Rollback capability prevents partial removals
5. **Batch Operations**: Efficient bulk removal with progress tracking

Once integrated into the CLI, users will be able to confidently manage their JDK installations with proper safeguards against common mistakes.