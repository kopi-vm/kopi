# Shims System Implementation Plan 2 - Gap Completion

## Overview
This document addresses the gaps between the original shims implementation plan (plan.md) and the current implementation status. It provides a focused plan to complete the remaining items for the Kopi shims core functionality.

## Scope Clarification
This plan focuses exclusively on completing the shims system implementation. The following are intentionally **out of scope**:
- Core JDK management commands (use, list, current, global, local, which) - These are separate from shims functionality
- Configuration commands - Not part of the current shims task
- Complex process executor enhancements - Basic implementation exists in `src/platform/process.rs`

The goal is to complete the shims system to provide transparent JDK version switching through intercepted tool invocations.

## Current Implementation Status Summary

### ✅ Completed Components
1. **Core shim runtime** (`src/shim/mod.rs`) - Tool detection, version resolution, execution
2. **Version resolver** (`src/shim/version_resolver.rs`) - Traverses directories for version files
3. **Shim installer** (`src/shim/installer.rs`) - Creates/removes/verifies shims
4. **Tool registry** (`src/shim/tools.rs`) - Comprehensive JDK tool definitions
5. **Error handling** (`src/shim/errors.rs`) - User-friendly error messages
6. **Shim commands** (`src/commands/shim.rs`) - add/remove/list/verify subcommands
7. **Platform support** - Symlinks (Unix) and file copying (Windows)
8. **Shell integration** (`src/platform/shell.rs`) - Shell detection and PATH configuration
9. **Performance optimizations** - release-shim profile in Cargo.toml
10. **Basic process execution** (`src/platform/process.rs`) - Platform-specific process handling

### ❌ Major Gaps to Address
1. **Post-install shim creation** - Automatic shim creation after JDK installation
2. **Security implementation** - Path validation, input sanitization, permission checks
3. **Performance benchmarks** - Measure shim overhead and optimization effectiveness
4. **Documentation updates** - Complete user reference and troubleshooting guide
5. **Modularization** - Extract auto-install logic from shim/mod.rs

## Phase 1: Post-Install Shim Creation

### Deliverables

1. **Install Command Enhancement** (`/src/commands/install.rs`)
   ```rust
   // After finalize_installation() call:
   if config.auto_create_shims {
       let shim_installer = ShimInstaller::new(&config)?;
       let jdk_path = storage.get_jdk_path(&jdk_info);
       let tools = discover_jdk_tools(&jdk_path)?;
       
       println!("\nVerifying shims...");
       let created_shims = shim_installer.create_missing_shims(&tools)?;
       
       if !created_shims.is_empty() {
           println!("Created {} new shims:", created_shims.len());
           for shim in &created_shims {
               println!("  - {}", shim);
           }
       }
   }
   ```

2. **JDK Tool Discovery** (`/src/shim/discovery.rs`)
   ```rust
   // Discover available tools in an installed JDK
   pub fn discover_jdk_tools(jdk_path: &Path) -> Result<Vec<String>> {
       let bin_dir = jdk_path.join("bin");
       // Scan for executables, filter by known tools from ToolRegistry
   }
   ```

### Success Criteria
- Shims automatically created after JDK installation (when configured)
- Distribution-specific tools detected and shims created
- User informed of newly created shims

## Phase 2: Modularization and Enhanced Testing

### Deliverables

1. **Auto-Install Module** (`/src/shim/auto_install.rs`)
   ```rust
   // Extract auto-install logic from shim/mod.rs
   pub struct AutoInstaller {
       config: Arc<KopiConfig>,
   }
   
   impl AutoInstaller {
       pub fn should_auto_install(&self) -> bool;
       pub fn prompt_user(&self, version: &str) -> Result<bool>;
       pub fn install_jdk(&self, version: &str) -> Result<()>;
   }
   ```

2. **Enhanced Configuration** (`/src/config.rs`)
   ```rust
   #[derive(Debug, Deserialize, Serialize)]
   pub struct ShimsConfig {
       pub additional_tools: Vec<String>,
       pub exclude_tools: Vec<String>,
       pub auto_install: bool,
       pub auto_install_prompt: bool,
       pub install_timeout: u64,
       pub auto_create_shims: bool,
   }
   ```

3. **Enhanced Unit Tests**
   - Mock-based tests for auto-install scenarios
   - Configuration loading and validation tests
   - Timeout handling tests

### Success Criteria
- Clean separation of auto-install logic
- Configuration fully controls shim behavior
- Timeout protection prevents hanging

## Phase 3: Security and Performance

### Deliverables

1. **Security Module** (`/src/shim/security.rs`)
   ```rust
   pub struct SecurityValidator;
   
   impl SecurityValidator {
       // Validate paths stay within ~/.kopi
       pub fn validate_path(&self, path: &Path) -> Result<()>;
       
       // Validate version strings (alphanumeric + @.-_)
       pub fn validate_version(&self, version: &str) -> Result<()>;
       
       // Validate tool names against registry
       pub fn validate_tool(&self, tool: &str) -> Result<()>;
       
       // Check file permissions
       pub fn check_permissions(&self, path: &Path) -> Result<()>;
   }
   ```

2. **Security Tests** (`/tests/shim_security.rs`)
   ```rust
   #[test]
   fn test_path_traversal_prevention() {
       // Test ../../../etc/passwd style attacks
   }
   
   #[test]
   fn test_symlink_target_validation() {
       // Test symlinks pointing outside ~/.kopi
   }
   
   #[test]
   fn test_version_string_validation() {
       // Test malicious version strings
   }
   
   #[test]
   fn test_permission_verification() {
       // Test non-executable files
   }
   ```

3. **Benchmark Suite** (`/benches/shim_bench.rs`)
   ```rust
   use criterion::{black_box, criterion_group, criterion_main, Criterion};
   
   fn benchmark_tool_detection(c: &mut Criterion) {
       c.bench_function("tool_detection", |b| {
           b.iter(|| detect_tool_name(black_box("java")))
       });
   }
   
   fn benchmark_version_resolution(c: &mut Criterion) {
       // Benchmark with cached vs file-based resolution
   }
   
   fn benchmark_total_overhead(c: &mut Criterion) {
       // End-to-end shim execution timing
   }
   ```

4. **Documentation Updates** (`/docs/reference.md`)
   - Add comprehensive shim command documentation
   - Include performance characteristics
   - Add security considerations section
   - Create troubleshooting guide for common shim issues

### Success Criteria
- All security tests pass
- Benchmarks confirm < 20ms overhead
- Binary size < 1MB verified
- Documentation complete and user-friendly

## Implementation Priority Order

### Priority 1: Post-Install Integration (3 days)
Improves user experience significantly:
1. Enhance install command with shim creation
2. Implement JDK tool discovery
3. Add configuration for auto-create behavior
4. Test across different JDK distributions

### Priority 2: Modularization (2 days)
Improves code maintainability:
1. Extract auto-install logic to separate module
2. Add comprehensive unit tests with mocks
3. Ensure clean separation of concerns

### Priority 3: Security Implementation (3 days)
Critical for production use:
1. Create security module with validation functions
2. Integrate validation into shim execution
3. Add comprehensive security tests
4. Verify no privilege escalation possible

### Priority 4: Performance Validation (2 days)
Ensures design goals are met:
1. Create benchmark suite
2. Measure shim overhead (target < 20ms)
3. Verify binary size < 1MB
4. Document performance characteristics

### Priority 5: Documentation and Polish (2 days)
Makes system production-ready:
1. Update reference.md with shim details
2. Add troubleshooting guide
3. Document security considerations
4. Clean up any remaining issues

## Testing Strategy Additions

### Integration Test Scenarios
1. **Shim Auto-Creation on Install**
   ```bash
   # Enable auto-create in config
   echo '[shims]' >> ~/.kopi/config.toml
   echo 'auto_create_shims = true' >> ~/.kopi/config.toml
   
   # Install GraalVM
   kopi install graalvm@21
   # Should automatically create gu, native-image shims
   which gu  # Should resolve to ~/.kopi/shims/gu
   ls -la ~/.kopi/shims/  # Should show all created shims
   ```

2. **Version Resolution with Shims**
   ```bash
   # Create project with version file
   mkdir test-project && cd test-project
   echo "temurin@21" > .kopi-version
   
   # Run Java through shim
   java -version  # Should use temurin 21
   
   # Test environment variable override
   KOPI_JAVA_VERSION=corretto@11 java -version  # Should use corretto 11
   ```

3. **Security Validation**
   ```bash
   # Attempt path traversal in version file
   echo "../../../etc/passwd@21" > .kopi-version
   java -version  # Should fail with security error
   
   # Test symlink validation
   ln -s /etc/passwd ~/.kopi/shims/malicious
   kopi shim verify  # Should detect and report invalid shim
   ```

## Migration Notes

### For Existing Users
1. After update, run `kopi setup` to ensure all shims are created
2. Check `~/.kopi/config.toml` for new configuration options
3. Existing `.java-version` files continue to work

### Breaking Changes
- None expected, all additions are backward compatible

## Success Metrics

### Functional Metrics
- All planned commands implemented and tested
- Shim system works transparently on all platforms
- Auto-installation reduces manual steps

### Performance Metrics
- Shim overhead consistently < 20ms
- Binary size < 1MB
- Cold start time < 10ms

### Quality Metrics
- > 90% test coverage for shim modules
- All security tests passing
- Documentation rated helpful by users

## Next Steps

1. Start with Priority 1 (Post-Install Integration) to improve user experience
2. Run `cargo fmt && cargo clippy && cargo test` after each module  
3. Ensure all existing tests continue to pass during refactoring
4. Get user feedback after Priority 2 (Modularization) completion
5. Focus on security and performance validation before final release