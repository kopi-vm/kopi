# `[Component/Feature]` Design

## Metadata

- Type: Design
- Status: Draft | In Review | Approved
  <!-- Draft: Work in progress | In Review: Awaiting technical review | Approved: Ready for implementation -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Requirements:
  - [FR-<id>-<name>](../requirements/FR-<id>-<name>.md)
  - [NFR-<id>-<name>](../requirements/NFR-<id>-<name>.md)
- Related ADRs:
  - [ADR-<id>-<title>](../adr/ADR-<id>-<title>.md)

## Overview

`[One-paragraph summary of the problem, motivation, and expected outcome.]`

## Success Metrics

- [ ] `[Measurable product/engineering impact]`
- [ ] `[Performance target (e.g., <X ms, <Y MB)]`
- [ ] `[Reliability target (e.g., zero regressions)]`

## Background and Current State

- Context: `[Where this fits in Kopi; user workflows it affects]`
- Current behavior: `[What exists today; relevant modules/paths]`
- Pain points: `[Current issues/limitations]`
- Constraints: `[Time/tech/platform/compliance]`
- Related ADRs: `[/docs/adr/0xx-...md]`

## Proposed Design

### High-Level Architecture

```text
[ASCII diagram of components and data flows]
```

### Components

- `[Modules/structs/functions and responsibilities]`

### Data Flow

- `[Sequence of operations from input to output]`

### Storage Layout and Paths (if applicable)

- JDKs: `~/.kopi/jdks/<vendor>-<version>/`
- Shims: `~/.kopi/shims/`
- Config: `~/.kopi/config.toml`
- Cache: `~/.kopi/cache/`

### CLI/API Design (if applicable)

Usage

```bash
kopi <command> `[options]`
```

Options

- `--flag`: `[Description]`
- `--option <value>`: `[Description]`

Examples

```bash
kopi <command> <example-1>
kopi <command> <example-2> --flag
```

Implementation Notes

- Use `clap` derive API for argument parsing with clear, English help messages.

### Data Models and Types

- `[Structs/enums/fields; serialization formats; version formats]`

### Error Handling

- Use `KopiError` variants with actionable, English messages.
- Integrate with `ErrorContext` for enriched output and correct exit codes.
- Exit codes: `[2 invalid input/config, 3 no local version, 4 JDK not installed, 13 permission, 20 network, 28 disk, 127 not found]`.

### Security Considerations

- `[HTTPS verification, checksum validation, unsafe path handling, permission checks]`

### Performance Considerations

- `[Hot paths; caching strategy; async/concurrency; I/O; progress indicators]`
- Reference perf workflows: `cargo perf`, `cargo bench`.

### Platform Considerations

#### Unix

- `[Paths/permissions/behavior; symlinks]`

#### Windows

- `[Registry/junctions; path separators; ACLs]`

#### Filesystem

- `[Case sensitivity; long paths; temp files]`

## Alternatives Considered

1. Alternative A
   - Pros: `[List]`
   - Cons: `[List]`
2. Alternative B
   - Pros: `[List]`
   - Cons: `[List]`

Decision Rationale

- `[Why chosen approach; trade-offs]`. Link/update ADR as needed.

## Migration and Compatibility

- Backward/forward compatibility: `[Behavior changes, flags, formats]`
- Rollout plan: `[Phased enablement, feature flags]`
- Telemetry/Observability (if any): `[What to measure; where logged]`
- Deprecation plan: `[Old commands/flags removal timeline]`

## Testing Strategy

### Unit Tests

- Place tests next to code with `#[cfg(test)]`; cover happy paths and edge cases.

### Integration Tests

- Add scenarios under `tests/`; avoid mocks; exercise CLI/IO boundaries.
- Use alias `cargo it` for quick runs.

### External API Parsing (if applicable)

- Include at least one unit test with captured JSON (curl) as an inline string parsed with `serde`; assert key fields.

### Performance & Benchmarks (if applicable)

- `cargo perf` (feature `perf_tests`) and `cargo bench`; define thresholds and compare trends.

## Documentation Impact

- Update `docs/reference.md` for CLI/behavior changes.
- Update user docs in `../kopi-vm.github.io/` if user-facing.
- Add or update `/docs/adr/` entries for design decisions (rationale and alternatives).

## External References (optional)

<!-- External standards, specifications, articles, or documentation -->

- [External resource title](https://example.com) - Brief description

## Open Questions

- [ ] `[Question that needs investigation]`
- [ ] `[Decision that needs to be made]` → Next step: `[Where to resolve (e.g., update plan docs/tasks/T-<id>-<task>/plan.md per TDL)]`
- [ ] `[Information that needs gathering]` → Method: `[How to obtain insight]`

<!-- Complex investigations should spin out into their own ADR or analysis document -->

## Appendix

### Diagrams

```text
[Additional diagrams]
```

### Examples

```bash
# End-to-end example flows
```

### Glossary

- Term: `[Definition]`

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](README.md#design-template-designmd) in the templates README.
