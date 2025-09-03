# Version Switching Commands Design

This directory contains the detailed design specifications for Kopi's version switching commands, based on ADR-016.

## Overview

Kopi provides three scopes for version management, each with its own command and priority level:

1. **Shell Scope** (highest priority) - Temporary version for current shell session
2. **Project Scope** (medium priority) - Version specific to a project directory
3. **Global Scope** (lowest priority) - System-wide default version

All version switching commands integrate with Kopi's auto-installation feature (configured in `~/.kopi/config.toml`), providing a seamless experience when working with different JDK versions.

## Command Structure

| Scope   | Primary Command                         | Alias Command                         | Priority | Persistence                     |
| ------- | --------------------------------------- | ------------------------------------- | -------- | ------------------------------- |
| Current | `kopi current`                          | -                                     | -        | Shows active version            |
| Shell   | `kopi shell <version> [--shell <type>]` | `kopi use <version> [--shell <type>]` | Highest  | Temporary (subprocess session)  |
| Project | `kopi local <version>`                  | `kopi pin <version>`                  | Medium   | Persistent (.kopi-version file) |
| Global  | `kopi global <version>`                 | `kopi default <version>`              | Lowest   | Persistent (~/.kopi/version)    |

## Version Resolution Order

When executing Java commands through shims, versions are resolved in this order:

1. `KOPI_JAVA_VERSION` environment variable (set by shell/use)
2. `.kopi-version` file in current or parent directories
3. `.java-version` file in current or parent directories
4. Global default from `~/.kopi/version`

If no version is found through any of these methods, the shim will error and suggest installing a JDK or setting a version.

## Design Documents

- [Current Command Design](./current-command.md) - Display active JDK version and resolution source
- [Shell Command Design](./shell-command.md) - Launches new shell subprocess with specified JDK version
- [Local Command Design](./local-command.md) - Project-specific version configuration
- [Global Command Design](./global-command.md) - System-wide default version management

## Common Design Principles

### Version Format

All commands support the following version formats:

- Simple version: `17`, `11`, `21`
- Full version: `17.0.5`, `11.0.2`
- Distribution with version: `temurin@17`, `openjdk@11.0.2`, `corretto@17.0.5`

### Error Handling

All commands follow these error handling principles:

- Clear error messages with actionable suggestions
- Validation of version availability before setting
- Graceful fallback when versions are not found
- Platform-specific guidance for environment setup

### User Experience

- Consistent command structure across all scopes
- Immediate feedback on version changes
- Support for both primary and alias commands
- Integration with `kopi current` for debugging version resolution

## Implementation Status

**Current Implementation State**:

- ✅ **Version Resolution**: Fully implemented in `src/version/resolver.rs` with proper priority order
- ✅ **Shim System**: Complete with auto-installation, security validation, and tool discovery
- ✅ **Platform Infrastructure**: Shell detection, process execution, and platform-specific utilities
- ⚠️ **CLI Commands**: Basic structure exists but commands not yet implemented:
  - `current`, `use`, `global`, `local`, `which` show "not yet implemented" messages
  - Only `install`, `cache`, `setup`, and `shim` commands are functional

**Integration with Shim Architecture**:

- Shims read environment variable and version files using `VersionResolver`
- Version resolution happens at command execution time
- Auto-installation provides seamless experience for missing versions
- Security validation ensures safe tool execution

## Auto-Installation Behavior

Commands have different behaviors when a non-existent version is specified:

| Command       | Auto-Install Support | Behavior                                                                 |
| ------------- | -------------------- | ------------------------------------------------------------------------ |
| `kopi shell`  | Yes                  | Prompts to install missing version (if enabled in config)                |
| `kopi local`  | Yes                  | Prompts to install missing version, creates version file regardless      |
| `kopi global` | Yes                  | Prompts to install missing version, only sets global if install succeeds |

This design allows:

- All commands to offer convenient auto-installation when enabled
- Local command to create version files even if installation is declined (for team collaboration)
- Global command to ensure the version is actually available before setting it as default
- Consistent behavior controlled by `~/.kopi/config.toml` settings
