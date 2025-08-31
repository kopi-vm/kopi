# [Component/Feature] Design

## Metadata
- Owner: [Person or role]
- Reviewers: [Names/roles]
- Status: [Draft / In Review / Approved]
- Last Updated: YYYY-MM-DD
- Links: [Requirements], [Issue], [PR], [ADR], [Implementation Plan], [Related Tasks]

## Overview

[One-paragraph summary of the problem, motivation, and expected outcome.]

### Success Metrics
- [ ] [Measurable product/engineering impact]
- [ ] [Performance target (e.g., <X ms, <Y MB)]
- [ ] [Reliability target (e.g., zero regressions)]

## Background and Current State

- Context: [Where this fits in Kopi; user workflows it affects]
- Current behavior: [What exists today; relevant modules/paths]
- Pain points: [Current issues/limitations]
- Constraints: [Time/tech/platform/compliance]
- Related ADRs: [`/docs/adr/0xx-...md`]

## Requirements Summary (from requirements.md)

- Refer to `docs/templates/requirements.md` (or task-specific requirements document).
- List referenced requirement IDs only; avoid duplicating full text.

Referenced Functional Requirements
- FR-###, FR-###, FR-###

Referenced Non-Functional Requirements
- NFR-### (performance), NFR-### (security), NFR-### (compatibility), etc.

## Proposed Design

### High-Level Architecture
```
[ASCII diagram of components and data flows]
```

### Components
- [Modules/structs/functions and responsibilities]

### Data Flow
- [Sequence of operations from input to output]

### Storage Layout and Paths (if applicable)
- JDKs: `~/.kopi/jdks/<vendor>-<version>/`
- Shims: `~/.kopi/shims/`
- Config: `~/.kopi/config.toml`
- Cache: `~/.kopi/cache/`

### CLI/API Design (if applicable)

Usage
```bash
kopi <command> [options]
```

Options
- `--flag`: [Description]
- `--option <value>`: [Description]

Examples
```bash
kopi <command> <example-1>
kopi <command> <example-2> --flag
```

Implementation Notes
- Use `clap` derive API for argument parsing with clear, English help messages.

### Data Models and Types
- [Structs/enums/fields; serialization formats; version formats]

### Error Handling
- Use `KopiError` variants with actionable, English messages.
- Integrate with `ErrorContext` for enriched output and correct exit codes.
- Exit codes: [2 invalid input/config, 3 no local version, 4 JDK not installed, 13 permission, 20 network, 28 disk, 127 not found].

### Security Considerations
- [HTTPS verification, checksum validation, unsafe path handling, permission checks]

### Performance Considerations
- [Hot paths; caching strategy; async/concurrency; I/O; progress indicators]
- Reference perf workflows: `cargo perf`, `cargo bench`.

### Platform Considerations

#### Unix
- [Paths/permissions/behavior; symlinks]

#### Windows
- [Registry/junctions; path separators; ACLs]

#### Filesystem
- [Case sensitivity; long paths; temp files]

## Alternatives Considered

1. Alternative A
   - Pros: [List]
   - Cons: [List]
2. Alternative B
   - Pros: [List]
   - Cons: [List]

Decision Rationale
- [Why chosen approach; trade-offs]. Link/update ADR as needed.

## Migration and Compatibility

- Backward/forward compatibility: [Behavior changes, flags, formats]
- Rollout plan: [Phased enablement, feature flags]
- Telemetry/Observability (if any): [What to measure; where logged]
- Deprecation plan: [Old commands/flags removal timeline]

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

## Implementation Plan

- Milestones/Phases: [Link to `docs/templates/plan.md` or task plan]
- Risks & Mitigations: [Top risks with mitigation/validation/fallback]

## Requirements Mapping

- Map requirements to design sections and tests for traceability.

| Requirement | Design Section | Test(s) / Benchmark(s) |
|-------------|----------------|-------------------------|
| FR-001 | [Section name] | tests/[...], unit #[...] |
| FR-002 | [Section name] | tests/[...], it #[...] |
| NFR-010 | Performance Considerations | bench: [...], perf #[...] |

## Documentation Impact

- Update `docs/reference.md` for CLI/behavior changes.
- Update user docs in `../kopi-vm.github.io/` if user-facing.
- Add or update `/docs/adr/` entries for design decisions (rationale and alternatives).

## Open Questions

- [Question] → [Owner] → [Due/next step]

## Appendix

### Diagrams
```
[Additional diagrams]
```

### Examples
```bash
# End-to-end example flows
```

### Glossary
- Term: [Definition]

---

## Template Usage Instructions

1. Replace placeholders across all sections; keep English for all documentation.
2. Link to relevant ADRs and create new ones when this design introduces material decisions.
3. Capture concrete acceptance/success metrics to enable verification.
4. Call out platform differences explicitly when touching shell, shims, filesystem, or paths.
5. Specify testing strategy early, including external API parsing tests if applicable.
6. Prefer clarity and safety over micro-optimizations; avoid `unsafe`, avoid vague names like "manager"/"util", and prefer functions for stateless behavior.
