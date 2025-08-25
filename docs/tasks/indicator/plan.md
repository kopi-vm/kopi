# Progress Indicator Module Implementation Plan

## Overview

This document outlines the implementation plan for creating a unified progress indicator system for Kopi. The implementation consolidates fragmented progress implementations into a consistent system with support for different environments (terminal, non-terminal, silent mode). The plan is divided into phases that can be completed independently with context resets (`/clear`) between each phase.

**Current Status**: Phase 1-12 completed ✅

## Phase 1: Core Trait and Structures ✅

**Goal**: Define the core trait and data structures for the progress indicator system.

### Input Materials
- **Documentation**:
  - `/docs/tasks/indicator/design.md` - Design specification
  - `/docs/reviews/2025-08-24-progress-indicator-locations.md` - Current implementation analysis

- **Source Code to Create**:
  - `/src/indicator/mod.rs` - Core trait and structures
  - `/src/indicator/types.rs` - Type definitions

### Tasks
- [x] Create `src/indicator/` directory structure
- [x] Define `ProgressIndicator` trait with required methods
- [x] Define `ProgressConfig` struct with fields:
  - [x] `operation: String`
  - [x] `context: String`
  - [x] `total: Option<u64>`
  - [x] `style: ProgressStyle`
- [x] Define `ProgressStyle` enum (Bytes, Count)
- [x] Add documentation comments for all public APIs
- [x] **Write unit tests**:
  - [x] Test struct construction
  - [x] Test default values
  - [x] Test Display implementations

### Deliverables
- `src/indicator/mod.rs` - Core module file with trait definition
- `src/indicator/types.rs` - Type definitions (ProgressConfig, ProgressStyle)
- Unit tests for type construction and validation
- API documentation for all public types

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::tests
cargo doc --no-deps --open
```

---

## Phase 2: Silent Implementation ✅

**Goal**: Implement the silent progress indicator using the Null Object pattern.

### Input Materials
- **Dependencies**:
  - Phase 1 (Core Trait and Structures)

- **Source Code to Create**:
  - `/src/indicator/silent.rs` - Silent implementation

### Tasks
- [x] Create `SilentProgress` struct
- [x] Implement `ProgressIndicator` trait for `SilentProgress`
- [x] Ensure all methods are no-ops (no output)
- [x] **Write unit tests**:
  - [x] Test that no panic occurs on any method call
  - [x] Test thread safety
  - [x] Test memory usage (should be minimal)

### Deliverables
- `src/indicator/silent.rs` - Complete silent implementation
- Unit tests verifying no-op behavior
- Documentation explaining when this implementation is used

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::silent::tests
```

---

## Phase 3: Simple Text Implementation ✅

**Goal**: Implement the simple text progress indicator for non-terminal environments.

### Input Materials
- **Dependencies**:
  - Phase 1 (Core Trait and Structures)

- **Source Code to Create**:
  - `/src/indicator/simple.rs` - Simple text implementation

### Tasks
- [x] Create `SimpleProgress` struct with state fields
- [x] Implement `ProgressIndicator` trait for `SimpleProgress`
- [x] Add start/complete message output with println!
- [x] Handle error messages with eprintln!
- [x] **Write unit tests**:
  - [x] Test message output format
  - [x] Test state management
  - [x] Test error handling
  - [x] Mock stdout/stderr for testing

### Deliverables
- `src/indicator/simple.rs` - Complete simple implementation
- Unit tests with output verification
- Documentation for CI/CD usage

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::simple::tests
# Manual testing
TERM=dumb cargo run -- cache refresh
```

---

## Phase 4: Indicatif Implementation ✅

**Goal**: Implement the full-featured progress indicator using the indicatif library.

### Input Materials
- **Dependencies**:
  - Phase 1 (Core Trait and Structures)
  - External crate: `indicatif = "0.17"`

- **Source Code to Create**:
  - `/src/indicator/indicatif.rs` - Indicatif implementation

### Tasks
- [x] Create `IndicatifProgress` struct
- [x] Implement `ProgressIndicator` trait for `IndicatifProgress`
- [x] Create template selection based on ProgressStyle and total
- [x] Configure progress bar/spinner with consistent styling:
  - [x] Progress chars: `█▓░`
  - [x] Spinner chars: `⣾⣽⣻⢿⡿⣟⣯⣷`
  - [x] Colors: Green spinner, cyan/blue bars
  - [x] Tick speed: 100ms
- [x] **Write unit tests**:
  - [x] Test progress bar creation
  - [x] Test spinner creation
  - [x] Test template selection logic
  - [x] Test update behavior

### Deliverables
- `src/indicator/indicatif.rs` - Complete indicatif implementation
- Unit tests for all progress types
- Visual consistency with existing progress bars

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::indicatif::tests
# Manual visual testing
cargo run -- install temurin@21
```

---

## Phase 5: Factory Implementation ✅

**Goal**: Implement the factory pattern for creating appropriate progress indicators.

### Input Materials
- **Dependencies**:
  - Phases 1-4 (All implementations)

- **Source Code to Create**:
  - `/src/indicator/factory.rs` - Factory implementation

### Tasks
- [x] Create `ProgressFactory` struct
- [x] Implement `create(no_progress: bool)` method
- [x] Add terminal detection logic using `std::io::stderr().is_terminal()`
- [x] Return appropriate implementation based on conditions
- [x] **Write unit tests**:
  - [x] Test factory returns correct implementation
  - [x] Test terminal detection
  - [x] Test no_progress flag handling
  - [x] Mock terminal detection for testing

### Deliverables
- `src/indicator/factory.rs` - Complete factory implementation
- Unit tests with mocked environment conditions
- Integration point for the rest of the application

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::factory::tests
```

---

## Phase 6: Status Reporter Implementation ✅

**Goal**: Implement the status reporter for consistent simple messages.

### Input Materials
- **Documentation**:
  - `/docs/tasks/indicator/design.md` - Status reporter design

- **Source Code to Create**:
  - `/src/indicator/status.rs` - Status reporter implementation

### Tasks
- [x] Create `StatusReporter` struct
- [x] Implement methods:
  - [x] `operation()` - Major operation messages
  - [x] `step()` - Step within operation
  - [x] `success()` - Success messages
  - [x] `error()` - Error messages (always shown)
- [x] Add silent mode support
- [x] **Write unit tests**:
  - [x] Test message formatting
  - [x] Test silent mode behavior
  - [x] Test error messages always shown

### Deliverables
- `src/indicator/status.rs` - Complete status reporter
- Unit tests for all message types
- Consistent message formatting across the application

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::status::tests
```

---

## Phase 7: Download Module Migration ✅

**Goal**: Migrate the download module to use the new progress indicator system.

### Input Materials
- **Source Code to Modify**:
  - `/src/download/progress.rs` - Current implementation
  - `/src/download/mod.rs` - Integration points

- **Dependencies**:
  - Phases 1-6 (Complete indicator system)

### Tasks
- [x] Replace `IndicatifProgressReporter` with new `ProgressIndicator`
- [x] Update `HttpFileDownloader` to use `ProgressFactory`
- [x] Remove old progress implementation
- [x] Update error handling
- [x] **Write integration tests**:
  - [x] Test download with progress
  - [x] Test download without terminal
  - [x] Test --no-progress flag
  - [x] Test byte formatting

### Deliverables
- Updated `src/download/progress.rs` using new system
- Removed legacy progress code
- Integration tests for download scenarios
- Consistent progress display for downloads

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib download::tests
# Manual testing
kopi install temurin@21
kopi install --no-progress liberica@21
```

---

## Phase 8: Cache Module Migration ✅

**Goal**: Migrate the cache module to use the new progress indicator system.

### Input Materials
- **Source Code to Modify**:
  - `/src/commands/cache.rs` - Cache command implementation

- **Dependencies**:
  - Phases 1-6 (Complete indicator system)

### Tasks
- [x] Replace direct `ProgressBar` usage with `ProgressIndicator`
- [x] Use factory for spinner creation
- [x] Update cache refresh progress
- [x] **Write integration tests**:
  - [x] Test cache refresh with progress
  - [x] Test spinner behavior
  - [x] Test --no-progress flag

### Deliverables
- Updated `src/commands/cache.rs` using new system
- Consistent spinner behavior
- Integration tests for cache operations

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib commands::cache::tests
# Manual testing
kopi cache refresh
kopi cache refresh --no-progress
```

---

## Phase 9: Uninstall Module Migration ✅

**Goal**: Migrate the uninstall module to use the new progress indicator system.

### Input Materials
- **Source Code to Modify**:
  - `/src/uninstall/progress.rs` - Current implementation
  - `/src/uninstall/batch.rs` - Batch operations

- **Dependencies**:
  - Phases 1-6 (Complete indicator system)

### Tasks
- [x] Replace custom `ProgressReporter` with new system
- [x] Update batch uninstall progress
- [x] Use Count style for batch operations
- [x] Add StatusReporter to uninstall module
- [x] Fix JavaFX version matching for uninstall
- [x] Remove incorrect uninstall failure suggestions
- [x] **Write integration tests**:
  - [x] Test single uninstall
  - [x] Test batch uninstall progress
  - [x] Test count formatting
  - [x] Test JavaFX uninstall

### Deliverables
- Updated `src/uninstall/progress.rs` using new system
- Consistent progress for batch operations
- Integration tests for uninstall scenarios
- Fixed JavaFX uninstall handling
- Cleaned up error suggestions

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib uninstall::tests
# Manual testing
kopi uninstall temurin@21
kopi uninstall --all
```

---

## Phase 10: Status Message Migration ✅

**Goal**: Migrate simple status messages to use StatusReporter.

### Input Materials
- **Source Code to Modify**:
  - `/src/commands/install.rs` - Installation messages
  - `/src/commands/setup.rs` - Setup messages
  - `/src/commands/shim.rs` - Shim messages
  - `/src/installation/auto.rs` - Auto-installation messages

- **Dependencies**:
  - Phase 6 (Status Reporter)

### Tasks
- [x] Replace println! statements with StatusReporter
- [x] Standardize message formatting
- [x] **Write integration tests**:
  - [x] Test message output
  - [x] Test message consistency

### Deliverables
- Updated command modules using StatusReporter
- Consistent message formatting across all commands

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib commands::tests
# Manual testing
kopi setup
kopi install temurin@21 --dry-run
```

---

## Phase 11: Global Flag Integration ✅

**Goal**: Add --no-progress as a global command-line flag that suppresses all progress indicators.

### Input Materials
- **Source Code to Modify**:
  - `/src/main.rs` - Add global CLI flag
  - `/src/commands/install.rs` - Add no_progress parameter
  - `/src/commands/uninstall.rs` - Add no_progress parameter  
  - `/src/uninstall/mod.rs` - Update UninstallHandler
  - `/src/uninstall/batch.rs` - Update BatchUninstaller
  - `/src/uninstall/progress.rs` - Update ProgressReporter
  - `/src/indicator/factory.rs` - Update ProgressFactory
  - All command modules - Thread no_progress through

- **Dependencies**:
  - Phases 1-10 (All migrations complete)

### Tasks
- [x] Add `--no-progress` as global flag in clap Parser
- [x] Add no_progress parameter to all command execute methods
- [x] Update ProgressReporter constructors to accept no_progress
- [x] Update UninstallHandler and BatchUninstaller with no_progress
- [x] Update ProgressFactory to handle no_progress mode
- [x] Thread no_progress parameter through all progress creation
- [x] **Write integration tests**:
  - [x] Test flag parsing
  - [x] Test flag propagation to all commands
  - [x] Test progress suppression when flag is set
  - [x] Test help text includes global flag

### Deliverables
- Global --no-progress flag available on all commands
- Updated command handlers accepting the flag
- Help text documentation for the flag

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib cli::tests
# Manual testing
kopi --help
kopi install --no-progress temurin@21
kopi cache refresh --no-progress
```

---

## Phase 12: Integration Testing ✅

**Goal**: Comprehensive integration testing of the progress indicator system.

### Dependencies
- Phases 1-11 complete

### Tasks
- [x] **Integration tests** for core workflows:
  - [x] Test install with different progress styles
  - [x] Test cache operations with progress
  - [x] Test batch operations with progress
  - [x] Test no-progress mode across all commands
- [x] **Environment tests**:
  - [x] Test terminal detection
  - [x] Test CI environment behavior (with serial tests for safety)
  - [x] Test pipe/redirect behavior
- [x] **Performance tests**:
  - [x] Verify minimal overhead
  - [x] Test memory usage
  - [x] Test with large operations

### Deliverables
- ✅ Comprehensive integration test suite in `tests/progress_indicator_integration.rs`
- ✅ Performance benchmarks in `benches/progress_indicator_bench.rs`
- ✅ Environment-specific test scenarios with `#[serial]` attribute for thread safety
- ✅ All tests passing with environment variable handling

### Verification
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib indicator::tests --quiet
cargo test --test progress_indicator_integration --quiet
cargo build --benches
# CI environment testing
CI=true cargo test --test progress_indicator_integration test_progress_in_ci_environment --quiet
```

---

## Phase 13: Documentation Updates

**Goal**: Update all documentation to reflect the new progress indicator system.

### Input Materials
- **Documentation to Update**:
  - `/docs/reference.md` - User documentation
  - `README.md` - Add --no-progress flag documentation
  - `/docs/tasks/indicator/design.md` - Mark as implemented

### Tasks
- [ ] Update user documentation:
  - [ ] Document --no-progress flag
  - [ ] Add troubleshooting for progress issues
  - [ ] Document CI/CD behavior
- [ ] Update developer documentation:
  - [ ] Document progress indicator architecture
  - [ ] Add migration guide for new commands
  - [ ] Document testing approach
- [ ] Create examples for common use cases

### Deliverables
- Updated user documentation with --no-progress flag
- Developer guide for using progress indicators
- Migration guide from old to new system
- Examples and best practices

### Verification
```bash
# Review documentation
cat README.md | grep -i progress
cat docs/reference.md | grep -i progress
# Generate and review API docs
cargo doc --no-deps --open
```

---

## Implementation Order

### Core Infrastructure (Phases 1-6)
1. **Phase 1**: Core Trait and Structures
2. **Phase 2**: Silent Implementation
3. **Phase 3**: Simple Text Implementation
4. **Phase 4**: Indicatif Implementation
5. **Phase 5**: Factory Implementation
6. **Phase 6**: Status Reporter Implementation

### Module Migration (Phases 7-10)
7. **Phase 7**: Download Module Migration
8. **Phase 8**: Cache Module Migration
9. **Phase 9**: Uninstall Module Migration
10. **Phase 10**: Status Message Migration

### Integration and Documentation (Phases 11-13)
11. **Phase 11**: Global Flag Integration
12. **Phase 12**: Integration Testing
13. **Phase 13**: Documentation Updates

## Dependencies

- External crates:
  - `indicatif = "0.17"` - Progress bar library
  - `std::io::IsTerminal` - Terminal detection (stable in Rust 1.70+)

- Existing modules:
  - `src/download/progress.rs` - Current download progress
  - `src/commands/cache.rs` - Cache command progress
  - `src/uninstall/progress.rs` - Uninstall progress
  - `src/metadata/generator.rs` - Metadata generation progress

## Risks & Mitigations

1. **Risk**: Breaking existing progress functionality
   - **Mitigation**: Implement one module at a time with tests
   - **Fallback**: Keep old implementation until new one is verified

2. **Risk**: Performance regression from abstraction
   - **Mitigation**: Benchmark before and after
   - **Fallback**: Optimize hot paths if needed

3. **Risk**: Terminal detection issues across platforms
   - **Mitigation**: Test on Windows, macOS, and Linux
   - **Fallback**: Add platform-specific workarounds

4. **Risk**: Inconsistent styling across modules
   - **Mitigation**: Centralized style constants
   - **Fallback**: Style configuration file

## Success Metrics

- [x] All existing progress indicators migrated (Phases 7-10 completed)
- [x] Consistent visual style across all operations
- [x] --no-progress flag works globally (Phase 11 completed)
- [x] No performance regression (< 1ms overhead - verified in Phase 12)
- [x] Works correctly in CI/CD environments (Silent mode tested with serial tests)
- [x] Comprehensive test coverage with integration tests and benchmarks (Phase 12)
- [x] Zero user-visible breaking changes

## Notes for Implementation

- Each phase is self-contained and can be completed independently
- Use `/clear` between phases to reset context if needed
- Commit working code at the end of each phase
- Run tests after each phase to ensure no regression
- Document any deviations from the plan in commit messages
- Keep backward compatibility until all modules are migrated