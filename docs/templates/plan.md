# [Feature/Task Name] Implementation Plan

## Metadata
- Owner: [Person or role]
- Reviewers: [Names/roles]
- Status: [Not Started / Phase X In Progress / Blocked / Under Review / Completed]
- Last Updated: YYYY-MM-DD
- Links: [Issue], [PR], [ADR], [Design], [Related tasks]

## Overview

[Brief description of the feature/task and its purpose]

### Scope
- Goal: [Outcome to achieve]
- Non-Goals: [Explicitly out of scope]
- Assumptions: [Operational/technical assumptions]
- Constraints: [Time/tech/platform/compliance]

### Success Metrics
- [ ] [Measurable success criterion]
- [ ] [Performance target if applicable]
- [ ] [User experience improvement]
- [ ] All existing tests pass; no regressions in [area]

### Plan Summary
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

### Change Log
- [Key decisions and reasons]

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

## Open Questions

- [Question] → [Owner] → [Due/next step]

---

## Visual/UI Reference (optional)
```
[ASCII diagram or example output]
```

---

## Template Usage Instructions

When using this template for a new feature:

1. Replace placeholders in all bracketed sections.
2. Adjust the number of phases based on complexity.
3. Break down tasks into specific, testable items.
4. Define verification commands and phase acceptance criteria.
5. Identify risks early, with mitigation and fallback.
6. Keep status updated as work progresses.
7. Phase independence: Ensure each phase is self-contained; the `/clear` command may be executed at phase boundaries to reset context.
8. Update or add ADRs when design decisions change.
