# ADR-020: Default Log Level Configuration

## Metadata
- Type: ADR (Lite)
- Owner: Developer Experience Team
- Reviewers: Core Team
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved and to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->
- Date Created: 2024-09-01

## Links
<!-- Internal project artifacts only. For external resources, see External References section -->
- Requirements: N/A – Quick fix based on user feedback
- Design: N/A – Simple configuration change
- Plan: N/A – Direct implementation
- Related ADRs: ADR-009 (Logging Strategy)
- Issue: #89
- PR: #92
- Supersedes: N/A – First version
- Superseded by: N/A – Current version

## Context
<!-- 2–4 bullets describing the problem, constraints, and scope. -->
- Users reported overly verbose default logs for common commands.
- Goal: Default to `INFO`; allow `DEBUG` via flag/env.
- Constraint: Must not break existing scripts using `KOPI_LOG` env var.

## Success Metrics (optional)
<!-- Simple success criteria if measurable -->
- User complaints about verbose output reduced to zero
- Review date: 2025-01-01

## Decision
We will set the default log level to `INFO` and enable `--verbose`/`-v` to elevate to `DEBUG`. Existing `KOPI_LOG` env var continues to override.

## Consequences
<!-- List the key outcomes, split into positives/negatives as needed. -->
- Positive: Cleaner default output; easier for new users.
- Positive: Keeps detailed logs available on demand.
- Negative: Some debug details require explicit `-v` now.

## Open Questions (optional)
<!-- Questions that arose during decision-making -->
- Should we add multiple verbosity levels (-v, -vv, -vvv)? → Core Team → Future enhancement

## External References (optional)
<!-- External standards, specifications, articles, or documentation only -->
- [Rust env_logger](https://docs.rs/env_logger) - Logging level configuration patterns

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.

