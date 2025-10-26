# T-s2g7h Active-Use Detection

## Metadata

- Type: Design
- Status: Draft
  <!-- Draft: Work in progress | Approved: Ready for implementation | Rejected: Not moving forward with this design -->

## Links

- Associated Plan Document:
  - [T-s2g7h-active-use-detection-plan](./plan.md)

## Overview

Kopi currently skips uninstall safety checks when a JDK is configured as the active default, allowing users to remove the version that their shell session or projects still require. This design introduces active-use detection in the uninstall flow so that Kopi aborts removal whenever the target JDK is set as the global default (`~/.kopi/version`) or referenced by the nearest project version file (`.kopi-version` / `.java-version`), unless the command explicitly uses `--force`. The update covers single and batch uninstall flows and ensures the error messaging guides users toward switching or forcing the uninstall.

## Success Metrics

- [ ] Single uninstall aborts with actionable messaging when removing the active global default without `--force`.
- [ ] Batch uninstall skips/removes only entries not active in the current session, surfacing per-JDK feedback for blocked removals.
- [ ] Integration tests cover global and project active detection for single and batch flows on Unix and Windows paths.

## Background and Current State

- Context: Kopi uninstall commands manage removal of installed JDKs via `UninstallHandler` (single) and `BatchUninstaller` (bulk).
- Current behavior: `perform_safety_checks` contains stubbed active-use detection that always returns false, so uninstall proceeds even if the JDK is the active default.
- Pain points: Removing the active JDK can break ongoing sessions, batch uninstall can delete globally pinned versions silently, and there is no guidance to switch or force instead.
- Constraints: Respect Traceable Development Lifecycle (TDL), avoid introducing new dependencies, keep detection synchronous for CLI responsiveness.
- Related ADRs: `/docs/adr/ADR-8mnaz-concurrent-process-locking-strategy.md` (locking expectations for uninstall operations).

## Proposed Design

### High-Level Architecture

```
CLI uninstall (single/batch)
    -> UninstallHandler / BatchUninstaller
        -> safety::perform_safety_checks(config, repo, jdk, context)
            -> ActiveUseDetector
                -> read global version file
                -> search nearest project version file
                -> compare against target InstalledJdk
            -> return ValidationError unless force override
```

### Components

- `uninstall::safety`: expose `perform_safety_checks` that receives `&KopiConfig`, `&JdkRepository`, a `&InstalledJdk`, and a contextual override flag.
- `ActiveUseDetector` (function helpers inside `safety.rs`):
  - `read_global_version_request` reads and parses `~/.kopi/version`.
  - `find_project_version_request` walks the working directory upwards to find `.kopi-version` or `.java-version`.
  - `request_matches_jdk` compares a `VersionRequest` with an `InstalledJdk` including optional distribution and JavaFX suffix.
- CLI command glue:
  - `UninstallHandler::uninstall_jdk` accepts a `force` flag and calls the updated safety checks.
  - `BatchUninstaller` propagates `force` into `execute_batch_removal` and per-JDK checks.
- Process detection (future enhancement boundary): extend the existing `platform::process` module (`src/platform/process.rs`) with `cfg`-gated helpers so uninstall safety checks can reuse a single cross-platform abstraction without introducing new submodules.

### Data Flow

1. CLI parses `--force` and passes it through to handler/batch APIs.
2. `perform_safety_checks` evaluates global and local active references.
3. If a match is found and `force` is false, return `KopiError::ValidationError` with actionable guidance.
4. For batch uninstall, collect per-JDK failures and continue processing the rest.

### Storage Layout and Paths (if applicable)

- Version files: `~/.kopi/version`, `<project>/.kopi-version`, `<project>/.java-version`.

### CLI/API Design (if applicable)

Existing flags remain; `--force` now overrides active-use detection while emitting an INFO-level notice that the active default is being removed intentionally.

### Data Models and Types

- Reuse `InstalledJdk` for target metadata.
- Parse version files into `VersionRequest` via existing parser to maintain consistent semantics (distribution filters, JavaFX suffix, etc.).

### Error Handling

- Return `KopiError::ValidationError` with messages:
  - Global: "Cannot uninstall <dist>@<version> because it is the global default (<path>). Use --force to override or run 'kopi global unset'."
  - Local: "Cannot uninstall ...; configured by <file>."
- When `--force` is present, emit `warn!` or reporter step to acknowledge risk and proceed.

### Security Considerations

- No new I/O beyond reading version files from user-controlled directories; ensure paths are normalized and errors are surfaced without leaking sensitive data.

### Performance Considerations

- File reads are small and cached by OS; detection runs once per uninstall target. No additional contention on locks.

### Platform Considerations

#### Unix

- Ensure path comparisons handle symlinks (use canonicalized paths when comparing JDK directories for `.java-version` resolutions).

#### Windows

- Handle CRLF in version files; rely on `trim()` from existing utilities. Ensure case-insensitive comparisons for distributions.

#### Filesystem

- Version files may contain Unicode; we treat content as UTF-8 consistent with existing parsing.

## Alternatives Considered

1. **Skip project detection until `kopi local` GA**
   - Pros: Less surface area now.
   - Cons: Leaves gap for project defaults already in use; fails requirement.
2. **Store active JDK metadata in lock state**
   - Pros: Single source of truth for active version.
   - Cons: Requires new persistence and coordination changes unrelated to uninstall safety.

Decision Rationale

- Reading existing version files is low cost, aligns with current resolver behavior, and meets requirements without structural changes.

## Migration and Compatibility

- Backward compatible: new validation only blocks uninstall in scenarios previously unsafe; users can override with `--force`.
- No change to file formats.
- No migration needed.

## Testing Strategy

### Unit Tests

- Add tests in `uninstall::safety` verifying matching logic for global and local version files, including JavaFX suffix and case sensitivity.
- Cover `request_matches_jdk` edge cases (distribution omitted, build components, etc.).

### Integration Tests

- Extend `tests/uninstall_integration.rs` to create version files and confirm uninstall errors without `--force` and succeeds with `--force`.
- Add batch uninstall scenario verifying partial failures when some targets are active.

### External API Parsing (if applicable)

- Not applicable; only local file parsing.

### Performance & Benchmarks (if applicable)

- Not required; file lookups negligible.

## Documentation Impact

- Update developer docs if necessary (internal README) to mention active-use guard.
- Coordinate with user docs repo (`../kopi-vm.github.io/`) to mention new uninstall guard messaging once implementation lands.

## External References (optional)

- N/A

## Open Questions

- [ ] Should `KOPI_JAVA_VERSION` environment overrides be treated as active for uninstall? → Resolve during plan by confirming requirement scope.
- [ ] How should we phrase reporter output when `--force` bypasses the guard to stay consistent with other feedback observers? → Decide in implementation.

## Appendix

### Diagrams

```
Uninstall CLI --force? --> Handler --> Safety Checks --> Global/Local Detection --> Proceed or Abort
```

### Examples

```bash
# Block uninstall when global default matches target
kopi uninstall temurin@21.0.5+11
# Suggest switching or forcing

# Override with force
kopi uninstall temurin@21.0.5+11 --force
```

### Glossary

- Active default: The JDK version currently selected by global or project configuration files.

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../../templates/README.md#design-template-designmd) in the templates README.
