# [Feature/Task Name] Implementation Plan

## Metadata

- Type: Implementation Plan
- Owner: [Person or role]
- Reviewers: [Names/roles]
- Status: Not Started / Phase X In Progress / Blocked / Under Review / Completed
  <!-- Not Started: Planning complete, awaiting start | Phase X In Progress: Actively working | Blocked: External dependency | Under Review: Implementation complete | Completed: All phases done and verified -->
- Date Created: YYYY-MM-DD

## Links

<!-- Internal project artifacts only. For external resources, see External References section -->

- Requirements: docs/tasks/T-<id>-<task>/requirements.md | N/A – <reason>
- Design: docs/tasks/T-<id>-<task>/design.md | N/A – <reason>
- Related ADRs: ADR-<id>, ADR-<id> | N/A – No related ADRs
- Issue: #XXX | N/A – <reason>
- PR: #XXX | N/A – <reason>

## Overview

[Brief description of the feature/task and its purpose]

## Success Metrics

- [ ] [Measurable success criterion]
- [ ] [Performance target if applicable]
- [ ] [User experience improvement]
- [ ] All existing tests pass; no regressions in [area]

## Scope

- Goal: [Outcome to achieve]
- Non-Goals: [Explicitly out of scope]
- Assumptions: [Operational/technical assumptions]
- Constraints: [Time/tech/platform/compliance]

## Plan Summary

- Phases: [Short list of phases and intent]
- Timeline (optional): [Milestones/estimates]

---

## Phase 1: [Core Component/Foundation]

### Goal

- [What this phase aims to achieve]

### Inputs

- Documentation:
  - `/docs/...` – [Purpose]
- Source Code to Modify:
  - `/src/...` – [Purpose]
  - `/src/...` – [Purpose]
- Dependencies:
  - Internal: `src/[module]/` – [Description]
  - External crates: `[crate_name]` – [Purpose]

### Tasks

- [ ] **[Task group]**
  - [ ] [Specific subtask]
  - [ ] [Specific subtask]
- [ ] **[Task group]**
  - [ ] [Specific subtask]
  - [ ] [Specific subtask]

### Deliverables

- [Artifacts/changes produced]

### Verification

```bash
# Build and checks
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
# Focused unit tests
cargo test --lib --quiet [module_name]
# If integration relevant
cargo it        # alias: test --quiet --features integration_tests
```

### Acceptance Criteria (Phase Gate)

- [Observable, testable criteria required to exit this phase]

### Rollback/Fallback

- [How to revert; alternative approach if needed]

---

## Phase 2: [Next Component]

### Goal

- [What this phase aims to achieve]

### Inputs

- Dependencies:
  - Phase 1: [Dependency description]
  - [Other dependencies]
- Source Code to Modify:
  - `/src/...` – [Purpose]

### Tasks

- [ ] **[Task group]**
  - [ ] [Specific subtask]
  - [ ] [Specific subtask]

### Deliverables

- [Artifacts/changes produced]

### Verification

```bash
cargo check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --lib --quiet [module_name]
# Optional: broader runs
cargo it
```

### Acceptance Criteria (Phase Gate)

- [Observable, testable criteria required to exit this phase]

### Rollback/Fallback

- [How to revert; alternative approach if needed]

---

## Phase 3: Testing & Integration

### Goal

- Create comprehensive tests and validate integration boundaries.

### Tasks

- [ ] Test utilities
  - [ ] [Helper functions]
  - [ ] [Fixtures/mocks as needed]
- [ ] Scenarios
  - [ ] Happy path
  - [ ] Error handling
  - [ ] Edge cases
- [ ] Concurrency & cleanup
  - [ ] Boundary conditions
  - [ ] Concurrent operations (honor RUST_TEST_THREADS=4)
  - [ ] Resource cleanup

### Deliverables

- Comprehensive automated tests for new behavior
- Documented known limitations and follow-ups (if any)

### Verification

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
# Integration tests
cargo it
# Full test suite (consider runtime)
cargo test --quiet
```

### Acceptance Criteria (Phase Gate)

- [Coverage of critical paths; green on unit + integration runs]

---

## Testing Strategy

### Unit Tests

- Place unit tests next to code using `#[cfg(test)]`. Focus on critical logic and edge cases.

### Integration Tests

- Add broader scenarios under `tests/`. Use `cargo it` alias for quick runs.

### External API Parsing (if applicable)

- Include at least one unit test with captured JSON from the real API (curl) stored inline as a string and parsed with serde; assert key fields.

### Performance & Benchmarks (if applicable)

- Perf tests: enable `perf_tests` feature. Run `cargo perf`.
- Benchmarks: run `cargo bench` and note trends/regressions.
- Avoid brittle assumptions around thread counts; tests run with `RUST_TEST_THREADS=4`.

---

## Platform Matrix (if applicable)

### Unix

- [Paths/permissions/behavior]

### Windows

- [Registry, junctions/symlinks, path separators]

### Filesystem

- [Case sensitivity, long paths]

---

## Dependencies

### External Crates

- `[crate_name]` – [Purpose]
- [Prefer minimal features where possible]

### Internal Modules

- `src/[module]/` – [Description]

---

## Risks & Mitigations

1. Risk: [Description]
   - Mitigation: [Plan]
   - Validation: [How to prove it works]
   - Fallback: [Alternative]

2. Risk: [Description]
   - Mitigation: [Plan]
   - Validation: [How to prove it works]
   - Fallback: [Alternative]

---

## Documentation & Change Management

### CLI/Behavior Changes

- Update `docs/reference.md` when commands, flags, or outputs change.
- If user-facing behavior changes, update user docs in `../kopi-vm.github.io/`.

### ADR Impact

- Add or update ADRs under `/docs/adr/` for material design decisions; include rationale and alternatives.

---

## Implementation Guidelines

### Error Handling

- Use `KopiError` variants; keep messages clear, actionable, and in English.
- Rely on the `ErrorContext` system; ensure correct exit codes for each error type.

### Naming & Structure

- Avoid vague terms like "manager" or "util". Prefer specific, descriptive names.
- Prefer functions for stateless behavior; introduce structs only when state/traits are required.

### Safety & Clarity

- Do not use `unsafe`. Prefer correct ownership and readability over micro-optimizations; avoid patterns like `Box::leak()`.

---

## Definition of Done

- [ ] `cargo check`
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --lib --quiet`
- [ ] Integration/perf/bench (as applicable): `cargo it`, `cargo perf`, `cargo bench`
- [ ] `docs/reference.md` updated; user docs updated if user-facing
- [ ] ADRs added/updated for design decisions
- [ ] Error messages actionable and in English; exit codes correct
- [ ] Platform verification completed (if platform-touching)
- [ ] No `unsafe` and no vague naming (no "manager"/"util")

---

## Status Tracking

- Not Started: Work hasn't begun
- Phase X In Progress: Currently working on a specific phase
- Phase X Completed: Phase finished; moving to next
- Blocked: Waiting on external dependency
- Under Review: Implementation complete; awaiting review
- Completed: All phases done and verified

---

## External References (optional)

<!-- External standards, specifications, articles, or documentation -->

- [External resource title](https://example.com) - Brief description

## Open Questions

- [Question] → [Owner] → [Due/next step]

---

## Visual/UI Reference (optional)

```
[ASCII diagram or example output]
```

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](README.md#plan-template-planmd) in the templates README.
