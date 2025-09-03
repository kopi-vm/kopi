# Shim Binary Implementation Design - Overview

## Overview

This document describes the design and implementation of Kopi's shim binary system for seamless JDK version switching. The shim system intercepts Java tool invocations and routes them to the appropriate JDK version based on project configuration.

## Goals

1. **Zero-overhead switching**: Minimize performance impact when invoking Java tools (target: 1-20ms overhead)
2. **Cross-platform compatibility**: Work seamlessly on Linux, macOS, and Windows
3. **Transparent operation**: Users should not notice shims are being used
4. **Automatic version detection**: Determine correct JDK version without user intervention
5. **Efficient process management**: Avoid unnecessary process chains through direct execution
6. **Security**: Only expose curated, user-facing tools through shims
7. **Maintainability**: Easy to update for new JDK distributions and tools

## Key Design Decisions

Based on our analysis and the insights from ADR-013, we've made the following key decisions:

1. **Shim Architecture**: Single compiled Rust binary (`kopi-shim`) with symlinks on Unix, individual `.exe` files on Windows
2. **Process Model**: Direct execution using `exec()` on Unix to avoid process chains
3. **Tool Selection**: Curated lists of user-facing tools, not automatic discovery
4. **Shim Lifecycle**: Shims created during setup and verified/updated during each JDK installation
5. **Performance Target**: 1-20ms overhead, achieved through minimal I/O and direct execution
6. **Automatic JDK Installation**: Missing JDKs are automatically installed when first accessed (configurable)

## Design Principles

We follow these principles for shim creation and management:

### 1. Explicit Over Implicit

Shims are created only through explicit user actions:

- During initial `kopi setup`
- Via `kopi shim add <tool>` command
- Through configuration files
- During `kopi install` for known distribution tools
- Never automatically based on user behavior

### 2. Predictable Behavior

Users should always know which shims exist:

- No surprise shim creation
- No dynamic shim generation during tool execution
- Clear commands to manage shims

### 3. Security

Only expose known, user-facing tools:

- Curated lists prevent exposure of internal JDK executables
- Reduces attack surface by limiting accessible tools

### 4. Consistency

Same shims work across all JDK versions:

- Standard tools are present in all JDK distributions
- Distribution-specific tools are explicitly documented

### 5. Graceful Degradation

When tools are unavailable:

- Clear error messages explain why a tool cannot be found
- Suggest alternatives (switch JDK, use different project)
- Never silently fail or use wrong version

## Implementation Example

The `kopi shim list --available` command allows users to discover what tools are available in their installed JDKs without automatically creating shims, maintaining the principle of explicit control.

This command would scan all installed JDKs, checking their bin directories for available executable tools. For each JDK, it would list all available tools and indicate whether a shim already exists for that tool. This gives users full visibility into what tools they could potentially create shims for, without automatically creating any shims.

The command output would show:

- The JDK distribution and version
- All executable tools found in that JDK's bin directory
- Whether a shim already exists for each tool
- Instructions on how to create shims for tools they want to use

This approach maintains user control while providing helpful discovery capabilities.

## Implications of These Principles

### Tool Availability

- A shim existing doesn't guarantee the tool is available in the current JDK
- Error messages must be helpful when tools are missing

### User Education

- Clear documentation about which tools belong to which distributions
- Command help text explains the shim system

### Project Portability

- Projects using distribution-specific tools may not be portable
- Warn users when they're using non-standard tools

### Performance

- Explicit shim creation means no dynamic overhead
- Predictable performance characteristics

## Anti-Patterns to Avoid

### 1. Automatic Shim Creation

Creating shims on-the-fly when a tool is first accessed would violate the principle of explicit control. Users should always consciously decide which tools to expose through shims.

### 2. Silent Fallbacks

When a tool isn't available in the current JDK, the system should never silently switch to a different JDK version or use an alternative tool. This would create unpredictable behavior and could lead to subtle bugs.

### 3. Overly Permissive Tool Lists

Exposing all executables found in a JDK's bin directory would create security risks and clutter. Many JDK executables are internal tools not meant for direct user access.

## Benefits of These Principles

1. **User Trust**: Users know exactly what Kopi is doing
2. **Debugging**: Clear error messages make troubleshooting easy
3. **Security**: Limited attack surface
4. **Performance**: No dynamic overhead
5. **Compatibility**: Works well with other tools

## Next: [Architecture](./02-architecture.md)
