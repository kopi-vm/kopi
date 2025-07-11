# ADR-016: Version Switching Command Design

## Status
Proposed

## Context
Kopi needs to provide commands for switching between JDK versions at different scopes. Based on comprehensive research of existing version management tools (nvm, pyenv, rbenv, rvm, sdkman), we need to design a consistent and predictable command structure that aligns with developer expectations while providing clear scope hierarchy.

The research identified three primary scopes for version activation:
- **Shell-specific (temporary)**: Active only in the current terminal session
- **Project-specific (local)**: Automatically activated based on project configuration files
- **User-specific (global/default)**: System-wide default for all sessions

## Decision

### Command Structure and Scope Hierarchy

Kopi will adopt the pyenv/rbenv model with clear scope precedence:

1. **Shell Scope** (highest priority)
   - Command: `kopi shell <version>` or `kopi use <version>`
   - Sets `KOPI_JAVA_VERSION` environment variable
   - Temporary, affects only current shell session
   - Takes precedence over all other settings

2. **Project Scope** (medium priority)
   - Command: `kopi local <version>` or `kopi pin <version>`
   - Creates `.kopi-version` file in current directory
   - Automatically activated when entering project directory
   - Also reads `.java-version` for compatibility

3. **Global Scope** (lowest priority)
   - Command: `kopi global <version>` or `kopi default <version>`
   - Sets system-wide default in `~/.kopi/version`
   - Used when no shell or project version is set

### Version Resolution Order

When executing a Java command through shims, Kopi will check in this order:
1. `KOPI_JAVA_VERSION` environment variable (set by `kopi shell/use`)
2. `.kopi-version` file in current directory or parent directories
3. `.java-version` file in current directory or parent directories
4. Global default from `~/.kopi/version`

If no version is found through any of these methods, the shim will error and suggest installing a JDK or setting a version.

### Command Aliases

To accommodate users from different ecosystems:
- `kopi use` → alias for `kopi shell` (familiar to nvm users)
- `kopi pin` → alias for `kopi local` (descriptive alternative)
- `kopi default` → alias for `kopi global` (familiar to sdkman users)

### Implementation with Shims

As decided in ADR-013, Kopi uses a shim-based approach. The version switching commands work by:
- **shell/use**: Setting environment variable that shims read
- **local/pin**: Creating version files that shims detect
- **global/default**: Writing to global configuration that shims fall back to

## Rationale

1. **Predictable Hierarchy**: The shell > local > global precedence is intuitive and widely adopted
2. **Compatibility**: Supporting `.java-version` files eases migration from other tools
3. **Flexibility**: Command aliases accommodate users from different ecosystems
4. **Simplicity**: Clear separation between temporary (shell) and persistent (local/global) changes
5. **Non-invasive**: Unlike rvm's approach, we don't modify shell built-ins or create complex functions

## Consequences

### Positive
- Clear mental model for users about scope and precedence
- Compatible with existing project configurations
- Predictable behavior across platforms
- Easy to debug version resolution issues
- Familiar commands for users of other version managers

### Negative
- Multiple aliases might cause initial confusion
- Need to maintain compatibility with both `.kopi-version` and `.java-version`
- Shell command requires setting environment variables (platform-specific handling)

## Implementation Notes

1. **Environment Variable Handling**:
   ```bash
   # Unix shells
   export KOPI_JAVA_VERSION=17
   
   # Windows Command Prompt
   set KOPI_JAVA_VERSION=17
   
   # Windows PowerShell
   $env:KOPI_JAVA_VERSION="17"
   ```

2. **Version File Format**:
   - Simple text file containing version string
   - One version per line (first line used)
   - Supports distribution@version format: `temurin@17.0.5`

3. **Command Examples**:
   ```bash
   # Temporary switch for current shell
   kopi use 17
   kopi shell temurin@17.0.5
   
   # Set project version
   kopi local 17
   kopi pin openjdk@11.0.2
   
   # Set global default
   kopi global 17
   kopi default corretto@17
   ```

4. **Debugging Support**:
   - `kopi current` shows active version and how it was resolved
   - `KOPI_DEBUG=1` environment variable for detailed resolution logging

## References
- Comprehensive analysis of version switching in nvm, pyenv, rbenv, rvm, and sdkman
- ADR-013: Binary Switching Approaches (shim implementation)
- pyenv documentation on version selection: https://github.com/pyenv/pyenv#choosing-the-python-version
- rbenv documentation on version precedence: https://github.com/rbenv/rbenv#how-it-works