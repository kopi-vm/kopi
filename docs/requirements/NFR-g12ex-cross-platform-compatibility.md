# NFR-g12ex Cross-Platform Lock Compatibility

## Metadata

- Type: Non-Functional Requirement
- Status: Approved
  <!-- Draft: Under discussion | Approved: Ready for implementation | Rejected: Decision made not to pursue this requirement -->

## Links

- Prerequisite Requirements:
  - N/A – No prerequisites
- Dependent Requirements:
  - [FR-02uqo-installation-locking](../requirements/FR-02uqo-installation-locking.md)
  - [FR-ui8x2-uninstallation-locking](../requirements/FR-ui8x2-uninstallation-locking.md)
  - [FR-v7ql4-cache-locking](../requirements/FR-v7ql4-cache-locking.md)
- Related Tasks:
  - [T-ec5ew-locking-foundation](../tasks/T-ec5ew-locking-foundation/README.md)

## Requirement Statement

Kopi SHALL deliver equivalent lock semantics and reliability across Linux, macOS, Windows, and WSL platforms using Rust standard library abstractions, ensuring consistent behaviour regardless of operating system or filesystem.

## Rationale

Cross-platform parity guarantees a predictable user experience, avoids divergent code paths, simplifies testing, and prevents subtle lock bugs when teams operate across multiple operating systems and filesystems.

## User Story (if applicable)

The system shall provide consistent locking across all supported platforms to ensure users receive the same behaviour regardless of operating system.

## Acceptance Criteria

- [ ] Lock acquisition, holding, and release semantics behave identically on Linux, macOS, Windows, and WSL environments.
- [ ] Standard library file locking APIs (`std::fs::File`) suffice for the core implementation; platform-specific branches exist only for fallback handling.
- [ ] Timeout mechanisms and error reporting return equivalent messages and exit codes across platforms.
- [ ] Supported filesystems include ext4, APFS, NTFS, and WSL ext4 with full lock support; detection occurs at runtime.
- [ ] Unsupported or degraded filesystems (for example, FAT32, certain network filesystems) trigger graceful fallback strategies (documented warnings plus atomic operations).
- [ ] CI coverage executes lock-focused tests on Linux, macOS, Windows, and WSL runners (or equivalent scheduled jobs).

## Technical Details (if applicable)

### Functional Requirement Details

N/A – Behavioural focus only.

### Non-Functional Requirement Details

- Compatibility: Maintain API parity and observable behaviour across supported operating systems and architectures.
- Reliability: Ensure lock lifecycle hooks (acquire/release) map to platform-specific APIs without divergence.
- Performance: Permit platform optimisations provided they do not change semantics or user-visible behaviour.
- Security: Normalise permission handling for lock files across platforms and enforce owner-only access.

## Platform Considerations

### Unix

- Use `flock(2)` via the standard library, handling `EINTR` and other POSIX nuances; detect network filesystems and degrade gracefully.

### Windows

- Use `LockFileEx` through the standard library; ensure errors translate consistently to `std::io::Error` values and support overlapped operations if required.

### Cross-Platform

- Detect filesystem characteristics (e.g., via `statfs`, `GetVolumeInformation`) to choose between advisory locks and fallback staging.
- Provide consistent logging and error messaging regardless of OS.

## Risks & Mitigation

| Risk                     | Impact | Likelihood | Mitigation                                                     | Validation                  |
| ------------------------ | ------ | ---------- | -------------------------------------------------------------- | --------------------------- |
| Platform API differences | High   | Low        | Rely on std library abstractions; maintain integration testing | CI matrix coverage          |
| Filesystem quirks        | Medium | Medium     | Detect filesystem type and adjust strategy                     | Filesystem acceptance tests |
| WSL compatibility issues | Low    | Medium     | Treat WSL as Linux but verify path handling                    | Include WSL scenarios in CI |

## Implementation Notes

- Avoid direct `#[cfg]` branches except within narrow shims encapsulated by a shared trait.
- Cache filesystem detection results to minimise repeated syscalls while still responding to environment changes.
- Surface platform and filesystem info in debug logs to aid support cases.
- Document fallback behaviour in the user-facing docs repository (`../kopi-vm.github.io/`).

## External References

- [`std::fs::File`](https://doc.rust-lang.org/std/fs/struct.File.html) – Rust standard library file locking behaviour

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](../templates/README.md#individual-requirement-template-requirementsmd) in the templates README.
