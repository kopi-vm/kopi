# FR-hq1ns Canonical Kopi Path Registry

## Metadata

- Type: Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - N/A – None yet
- Dependent Requirements:
  - N/A – None yet
- Related Tasks:
  - [T-wn8p3-path-registry](../tasks/T-wn8p3-path-registry/README.md)
- Related Analyses:
  - [AN-uqva7-path-registry-consolidation](../analysis/AN-uqva7-path-registry-consolidation.md)

## Requirement Statement

> Focus the requirement on the problem to solve and the desired outcome, remaining independent of any specific implementation approach.

Kopi SHALL expose a canonical set of path helpers under `src/paths/` that, given a Kopi home root, returns the absolute paths for all supported installation, cache, shim, locking, configuration, and telemetry artefacts while encapsulating input normalisation and required directory provisioning.

## Rationale

Hard-coded directory segments and ad hoc sanitisation appear in multiple subsystems today, increasing drift risk and complicating audits. Centralising the behaviour ensures consistent lock placement, predictable on-disk layout, and reduces duplication for future features that need new directories or files.

## User Story (if applicable)

As a Kopi maintainer, I want a single module that hands back correctly normalised Kopi home paths so that new features can reuse the same layout rules without duplicating string concatenation or directory creation logic.

## Acceptance Criteria

- [ ] All Kopi home path derivations for JDK installs, cache artefacts, shims, lock files, configuration state, and version markers are provided by functions within `src/paths/`.
- [ ] Path helpers enforce or apply the same sanitisation rules currently handled by `PackageCoordinate::slug` and `sanitize_segment`, eliminating duplicate slugging logic in downstream modules.
- [ ] Directory-creating helpers return an error context compatible with `KopiError::ConfigError` when creation fails and are covered by unit tests demonstrating success and error propagation.
- [ ] Call sites previously constructing these paths directly (e.g., storage repository, shim installer, doctor checks, locking scope) are updated to consume the canonical helpers with regression tests confirming no path changes.

## Technical Details (if applicable)

### Functional Requirement Details

- `src/paths/` may organise helpers by domain (installation, cache, shims, locking, config) but must present a stable public interface exported via `paths::` re-exports.
- Directory creation responsibilities can be provided as explicit `ensure_*` helpers or flags; the API must document when directories are created vs. read-only.
- Slugging utilities must reside in modules reachable without introducing cyclic dependencies; consider extracting sanitisation helpers into a shared submodule that both locking and paths consume.

### Non-Functional Requirement Details

N/A – See companion compatibility requirement.

## Platform Considerations

### Unix

- Helpers MUST preserve existing directory names (e.g., `jdks`, `cache`, `shims`, `locks`) and keep permissions handling delegated to callers.

### Windows

- Helpers MUST honour Windows-safe path segments and ensure sanitisation strips characters invalid on NTFS while matching existing slug behaviour.

### Cross-Platform

- Functions MUST accept absolute `KOPI_HOME` roots and work with UNC paths, WSL mounts, and case-insensitive filesystems without emitting mixed separators.

## Risks & Mitigation

| Risk                                                     | Impact | Likelihood | Mitigation                                                           | Validation                              |
| -------------------------------------------------------- | ------ | ---------- | -------------------------------------------------------------------- | --------------------------------------- |
| Overlooked call sites continue constructing paths ad hoc | Medium | High       | Audit with `rg` searches and add reviewer checklist coverage         | Integration diff comparing before/after |
| New module introduces circular dependencies              | Medium | Medium     | Keep sanitisation helpers in dedicated modules, avoid config imports | Clippy + cargo check for cyclic imports |
| Directory creation semantics diverge across callers      | High   | Medium     | Document creation behaviour and supply explicit "ensure" helpers     | Unit tests covering creation failures   |

## Implementation Notes

- Update traceability so T-wn8p3 references this requirement once approved.
- Consider exposing `enum` or struct wrappers for frequently used paths (e.g., `InstallPaths`) if it simplifies testing.
- Add targeted unit tests in `src/paths/` validating slugging, sanitisation fallbacks, and directory creation side effects using `tempfile`.

## External References

- N/A – No external references.

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
