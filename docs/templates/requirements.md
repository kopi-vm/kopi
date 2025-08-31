# [Component/Feature] Requirements

## Metadata
- Owner: [Person or role]
- Reviewers: [Names/roles]
- Status: [Draft / In Review / Approved]
- Last Updated: YYYY-MM-DD
- Links: [Issue], [PR], [ADR], [Design], [Implementation Plan]

## Problem Statement

[Brief description of the user problem, motivation, and value. Who is impacted and why this matters now.]

## Objectives & Success Metrics

- [ ] Objective 1: [SMART metric, e.g., “render <150 ms for 10k rows”]
- [ ] Objective 2: [Measurable UX/quality target]
- [ ] No regressions in existing functionality

## Scope

### Goals
- [Concrete outcomes this work must achieve]

### Non-Goals
- [Explicitly out of scope to avoid drift]

### Assumptions
- [Technical/organizational assumptions]

### Constraints
- [Time/tech/platform/policy constraints]

## Stakeholders & Personas (optional)

- [Primary user], [Secondary stakeholders]

## User Stories / Use Cases

- As a [persona], I want [capability], so that [benefit]. (FR-001)
- As a [persona], I want [capability], so that [benefit]. (FR-002)

## Functional Requirements (FR)

Use IDs and priority tags for traceability (e.g., FR-001 [Must], FR-010 [Should]).

- FR-001 [Must]: [Requirement text]
- FR-002 [Must]: [Requirement text]
- FR-010 [Should]: [Requirement text]
- FR-020 [Could]: [Requirement text]

## Non-Functional Requirements (NFR)

- NFR-001 [Performance]: [Target and measurement method]
- NFR-002 [Security]: [TLS/verification, permissions, checksum validation]
- NFR-003 [Reliability]: [Retry strategy, idempotency, failure modes]
- NFR-004 [Compatibility]: [Unix/Windows/filesystem specifics]
- NFR-005 [UX]: [English messages, help clarity, CLI ergonomics]
- NFR-006 [Observability] (optional): [Logs, metrics]

## CLI/UX Requirements (if applicable)

### Command Syntax
```bash
kopi <command> [options]
```

### Options
- `--flag`: [Description]
- `--option <value>`: [Description]

### Examples
```bash
kopi <command> <example-1>
kopi <command> <example-2> --flag
```

### Help & Messages
- English only; concise and actionable.

## Data/API Requirements (if applicable)

- Data models: [Key fields and formats]
- External API: [Endpoints, parameters]
- Include at least one captured JSON example for parsing tests.

## Platform Matrix

### Unix
- [Paths/permissions/behavior]

### Windows
- [Registry/junctions; path separators; ACLs]

### Filesystem
- [Case sensitivity; long paths; temp files]

## Dependencies

- Internal modules: `src/[module]/` – [Role]
- External crates: `[name]` – [Purpose], with minimal features enabled

## Risks & Mitigations

1. Risk: [Description]
   - Mitigation: [Plan]
   - Validation: [How to verify]
   - Fallback: [Alternative]

## Acceptance Criteria

- Criteria reference FR/NFR IDs and are objectively verifiable.
- [ ] Satisfies FR-001 (measured by …)
- [ ] Satisfies FR-002 (measured by …)
- [ ] Meets NFR-001 (<= X ms on …)
- [ ] Error messages: English, actionable, correct exit codes (NFR-005)

## Verification Plan

- Unit tests: `cargo test --lib --quiet` (link to test IDs)
- Integration tests: `cargo it` scenarios covering FR-###
- Performance: `cargo perf`, `cargo bench` thresholds for NFR-###
- Platform: Verification notes for Unix/Windows/filesystem

## Traceability (optional)

| Requirement | Design Section | Test(s) / Benchmarks | Status |
|-------------|----------------|----------------------|--------|
| FR-001 | [Design §] | tests/[...], it #[...] | Pending |
| NFR-001 | [Design §] | bench: [...], perf #[...] | Pending |

## Open Questions

- [Question] → [Owner] → [Due/next step]

## Change Log

- YYYY-MM-DD: [Change summary]

---

## Template Usage Instructions

1. Define FR/NFR with IDs and measurable criteria; keep brief and testable.
2. Link this requirements doc from the corresponding Design and Plan documents.
3. Keep acceptance criteria and verification here; Design should reference FR/NFR rather than duplicate.
4. Prefer clarity and safety: English-only messaging, avoid "manager"/"util" naming, do not use `unsafe`.
