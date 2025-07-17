# Kopi Env Command Implementation Plan

## Overview

This document outlines the implementation plan for the `kopi env` command, which outputs environment variables for shell evaluation to set up Java development environments.

## Phase 1: Core Functionality

### 1.1 Command Infrastructure
- [ ] Add `EnvCommand` struct in `src/commands/env.rs`
- [ ] Define command arguments using clap derive API:
  - Optional `<version>` argument
  - `--shell` option
  - `--export` option  
  - `--quiet` option
- [ ] Register command in `src/commands/mod.rs` and main CLI

### 1.2 Version Resolution
- [ ] Integrate with existing `resolve_version` functionality
- [ ] Handle version resolution hierarchy:
  1. Command line argument
  2. `KOPI_JAVA_VERSION` environment variable
  3. `.kopi-version` file (walk up directory tree)
  4. `.java-version` file (walk up directory tree)
  5. Global configuration
- [ ] Return `NoLocalVersion` error when no version found

### 1.3 JDK Validation
- [ ] Use `JdkInstallation::find` to verify JDK exists
- [ ] Return `JdkNotInstalled` error if missing
- [ ] Extract JDK home directory path

### 1.4 Basic Shell Formatting
- [ ] Create `EnvFormatter` trait with `format_env` method
- [ ] Implement for Bash/Zsh (same format):
  ```bash
  export JAVA_HOME="/path/to/jdk"
  ```
- [ ] Handle platform-specific path separators

### 1.5 Error Handling
- [ ] Use existing `KopiError` types
- [ ] Integrate with `ErrorContext` for helpful messages
- [ ] Implement `--quiet` flag to suppress stderr output

## Phase 2: Multi-Shell Support

### 2.1 Shell Detection Integration
- [ ] Use existing `platform::shell::detect_shell()` function
- [ ] Handle `--shell` option with `parse_shell_name()`
- [ ] Pass detected/specified shell to formatter

### 2.2 Fish Shell Support
- [ ] Implement Fish formatter:
  ```fish
  set -gx JAVA_HOME "/path/to/jdk"
  ```
- [ ] Add Fish-specific tests

### 2.3 PowerShell Support
- [ ] Implement PowerShell formatter:
  ```powershell
  $env:JAVA_HOME = "C:\path\to\jdk"
  ```
- [ ] Handle Windows path formats
- [ ] Add PowerShell-specific tests

### 2.4 Windows CMD Support
- [ ] Implement CMD formatter:
  ```cmd
  set JAVA_HOME=C:\path\to\jdk
  ```
- [ ] Handle `--export` flag (no-op for CMD)
- [ ] Add CMD-specific tests

## Phase 3: Performance Optimization

### 3.1 Initial Benchmarking
- [ ] Create benchmark suite in `benches/env_command.rs`
- [ ] Measure scenarios:
  - Cold start with global config
  - Project directory with `.kopi-version`
  - Deep directory hierarchy
  - Error cases
- [ ] Use `hyperfine` for real-world timing

### 3.2 Performance Analysis
- [ ] Profile with `cargo flamegraph`
- [ ] Identify bottlenecks:
  - Binary startup time
  - Config parsing overhead
  - File system operations
  - Version resolution logic

### 3.3 Optimization Implementation
- [ ] If performance < 100ms target:
  - Lazy loading of unused modules
  - Optimize config parsing
  - Cache file system lookups
  - Consider mmap for config files

### 3.4 Alternative Binary Decision
- [ ] If optimizations insufficient:
  - Design `kopi-env` minimal binary
  - Remove network dependencies
  - Strip unused features
  - Implement with minimal crate dependencies

## Phase 4: Testing & Integration

### 4.1 Unit Tests
- [ ] Version resolution edge cases
- [ ] Shell detection accuracy
- [ ] Formatter output correctness
- [ ] Error handling scenarios
- [ ] Path escaping/quoting

### 4.2 Integration Tests
- [ ] Test with real shells:
  - Bash eval execution
  - Zsh eval execution  
  - Fish source execution
  - PowerShell invocation
- [ ] Verify environment variable setting
- [ ] Test with various JDK installations

### 4.3 Documentation
- [ ] Update reference.md with actual implementation
- [ ] Add examples to CLI help text
- [ ] Create shell integration guide
- [ ] Document performance characteristics

## Implementation Order

1. **Week 1**: Phase 1 - Core functionality with bash/zsh support
2. **Week 2**: Phase 2 - Multi-shell support
3. **Week 3**: Phase 3 - Performance benchmarking and optimization
4. **Week 4**: Phase 4 - Comprehensive testing and documentation

## Success Criteria

- [ ] Command executes in < 100ms for typical use cases
- [ ] Supports all major shells (bash, zsh, fish, PowerShell, cmd)
- [ ] Integrates seamlessly with existing error handling
- [ ] Passes all unit and integration tests
- [ ] Documentation is complete and accurate

## Dependencies

- Existing modules:
  - `platform::shell` for shell detection
  - `version::resolve_version` for version resolution
  - `error` module for error types and handling
  - `models::JdkInstallation` for JDK validation

## Risks & Mitigations

1. **Performance Risk**: Shell hooks require fast execution
   - Mitigation: Benchmark early, consider separate binary if needed

2. **Shell Compatibility**: Different shells have varying syntax
   - Mitigation: Comprehensive testing on each platform

3. **Path Escaping**: Special characters in paths could break shell evaluation
   - Mitigation: Proper escaping/quoting for each shell type