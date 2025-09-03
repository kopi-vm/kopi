# Version Management Tools Research

## Overview

This document summarizes research on how popular version management tools handle default versions and symlinks, conducted to inform Kopi's design decisions.

## Tools Analyzed

### 1. SDKMAN!

**Symlink Approach**: Creates a "current" symlink

- Path: `~/.sdkman/candidates/java/current`
- Points to: Currently active Java version directory
- Updated: When running `sdk default java <version>` or `sdk use java <version>`
- Purpose: Provides a stable path for the active version

Example structure:

```
~/.sdkman/candidates/java/
├── 8u141-oracle/
├── 8u144-zulu/
├── 9ea14-zulu/
└── current -> /home/user/.sdkman/candidates/java/8u144-zulu/
```

### 2. rbenv, pyenv, and jenv

**Approach**: Shim-based architecture without default symlinks

- Shim directories: `~/.rbenv/shims/`, `~/.pyenv/shims/`, `~/.jenv/shims/`
- Global version files: `~/.rbenv/version`, `~/.pyenv/version`, `~/.jenv/version`
- No symlinks to "default" or "current" versions
- Shims dynamically resolve versions at runtime

Version resolution priority:

1. Environment variable (e.g., `RBENV_VERSION`)
2. Local version file (`.ruby-version`)
3. Global version file (`~/.rbenv/version`)
4. System version

### 3. nvm (Node Version Manager)

**Approach**: Optional current symlink

- Can create `$NVM_DIR/current` when `NVM_SYMLINK_CURRENT=true`
- Disabled by default to avoid race conditions
- Windows version uses persistent system-wide symlinks

### 4. Volta

**Approach**: Hybrid shim with symlinks

- Creates symlinks that all point to a single intelligent shim
- Example: `node -> volta-shim`, `npm -> volta-shim`
- The shim determines correct version based on project context
- Written in Rust for performance

## Key Findings

### Static vs Dynamic Symlinks

1. **Static symlinks** (pointing to a global default):
   - Not commonly used by modern version managers
   - Can conflict with project-specific settings
   - Example issue: `JAVA_HOME=~/.kopi/default` conflicts with project-specific versions

2. **Dynamic symlinks** (updated based on context):
   - SDKMAN!'s `current` reflects the active session
   - More flexible but requires frequent updates

3. **Shim-based approaches**:
   - Most common pattern (rbenv, pyenv, jenv)
   - No symlinks needed
   - Version resolution happens at command execution

## JAVA_HOME Conflict Issue

When setting `JAVA_HOME=~/.kopi/default`:

- Global default: Java 17 (`~/.kopi/default` → `~/.kopi/jdks/17`)
- Project setting: `.kopi-version` specifies Java 21
- Result: `java` command uses Java 21 (via shim), but `JAVA_HOME` points to Java 17
- Build tools (Maven/Gradle) use `JAVA_HOME`, causing version mismatch

## Recommendations for Kopi

Based on this research:

1. **Avoid static default symlinks**: The `~/.kopi/default` symlink creates more problems than it solves
2. **Use version files**: Follow rbenv/pyenv pattern with `~/.kopi/version` for global defaults
3. **Rely on shims**: Let shims handle version resolution dynamically
4. **Dynamic JAVA_HOME**: Consider shell integration that updates `JAVA_HOME` based on current context

## Conclusion

Static symlinks pointing to a global default are not a common pattern in modern version management tools. The potential for conflicts with project-specific settings outweighs any benefits. Kopi should follow the established pattern of using version files and shims for version management.
