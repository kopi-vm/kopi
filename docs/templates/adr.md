# `[Concise Decision Title]`

## Metadata

- Type: ADR
- Owner: `[Person or role who proposed this]`
- Reviewers: `[Names/roles who reviewed]`
- Status: Proposed / Accepted / Rejected / Deprecated / Superseded by ADR-<id>
  <!-- Proposed: Under discussion | Accepted: Approved and to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->

## Links

<!-- Internal project artifacts only. The Links section is mandatory for traceability. If a link does not apply, use "N/A – <reason>". -->

- Analysis: AN-<id> | N/A – <reason>
- Requirements: FR-<id>, NFR-<id> | N/A – <reason>
- Design: docs/tasks/T-<id>-<task>/design.md | N/A – <reason>
- Plan: docs/tasks/T-<id>-<task>/plan.md | N/A – <reason>
- Related ADRs: ADR-<id>, ADR-<id> | N/A – No related ADRs
- Issue: #XXX | N/A – <reason>
- PR: #XXX | N/A – <reason>
- Supersedes: ADR-<id> | N/A – First version
- Superseded by: ADR-<id> | N/A – Current version

## Context

<!-- What problem or architecturally significant requirement motivates this decision? Include constraints, assumptions, scope boundaries, and prior art. Keep value-neutral and explicit. -->

- What problem are we solving?
- What constraints/assumptions apply?
- What forces are in tension?
- What pain points or limitations exist?

## Success Metrics (optional)

<!-- Define measurable criteria to evaluate if this decision was successful -->

- Metric 1: `[what to measure, e.g., "API response time < 200ms"]`
- Metric 2: `[target value, e.g., "Error rate < 1%"]`
- Metric 3: `[qualitative measure, e.g., "Developer feedback positive"]`
- Review date: `[YYYY-MM-DD when to evaluate]`

## Decision

<!-- State the decision clearly in active voice. Start with "We will..." or "We have decided to..." and describe the core rules, policies, or structures chosen. Include short examples if clarifying. -->

### Decision Drivers (optional)

- `[criterion 1]`
- `[criterion 2]`

### Considered Options (optional)

- Option A: <name>
- Option B: <name>
- Option C: <name>

### Option Analysis (optional)

- Option A — Pros: <…> | Cons: <…>
- Option B — Pros: <…> | Cons: <…>
- Option C — Pros: <…> | Cons: <…>

## Rationale

<!-- Explain why this decision was made. Tie back to drivers and context. Be explicit about trade-offs and why alternatives were not chosen. -->

## Consequences

### Positive

- `[benefit 1]`
- `[benefit 2]`

### Negative

- `[cost/limitation 1]`
- `[cost/limitation 2]`

### Neutral (if useful)

- `[neutral effect or caveat]`

## Implementation Notes (optional)

- High-level plan, phases, or priority for adopting the decision.
- Interfaces/CLI/UX conventions (args, flags, exit codes) if part of the decision.
- Storage paths, data models, and error handling patterns (reference KopiError/ErrorContext if applicable).

## Examples (optional)

```bash
# Example commands or usage that illustrate the decision
kopi <command> `[args]`
```

```rust
// Code/pseudocode illustrating core mechanics
```

## Platform Considerations (required if applicable)

- Unix/Windows/filesystem behavior, libc variants, architectures, and any auto-detection or compatibility handling.

## Security & Privacy (required if applicable)

- Sensitive data handling, logging limitations, threat considerations.

## Monitoring & Logging (required if applicable)

- Verbosity model, environment variables, diagnostics guidance, and observability hooks.

## Open Questions

<!-- Questions that arose during decision-making but don't block the decision -->

- `[Question]` → `[Owner]` → `[Due/next step]`

## External References (optional)

<!-- External standards, specifications, articles, or documentation only -->

- [ADR GitHub](https://adr.github.io/) - ADR methodology documentation
- [External resource title](https://example.com) - Brief description

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.
