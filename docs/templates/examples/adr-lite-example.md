# Default Log Level Configuration

## Metadata

- Type: ADR (Lite)
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved and to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Analyses:
  - N/A – Issue identified directly from support feedback
- Related Requirements:
  - [FR-58sja-cli-log-level](../../requirements/FR-58sja-cli-log-level.md)
- Related ADRs:
  - [ADR-8b2pt-logging-strategy](../../adr/ADR-8b2pt-logging-strategy.md)
- Related Tasks:
  - [T-h9q7d-log-tuning](../../tasks/T-h9q7d-log-tuning/README.md)

## Context

<!-- 2–4 bullets describing the problem, constraints, and scope. -->

- Users reported overly verbose default logs for common commands.
- We must maintain compatibility with the existing `KOPI_LOG` environment variable.
- CLI output should remain quiet for success paths while still exposing debug detail on demand.

## Success Metrics (optional)

<!-- Simple success criteria if measurable -->

- Support tickets about noisy logs drop to zero within one release cycle.
- Review date: 2025-01-01

## Decision

We will set the default log level to `INFO` and rely on `--verbose`/`-v` or `KOPI_LOG=debug` to elevate verbosity when required.

## Consequences

<!-- List the key outcomes, split into positives/negatives as needed. -->

- Positive: Cleaner default output for first-time users and scripts.
- Positive: Retains explicit control for advanced debugging via flags or environment variables.
- Negative: Some diagnostics now require an additional flag to display.

## Open Questions (optional)

<!-- Questions that arose during decision-making -->

- Should we add multiple verbosity levels (`-v`, `-vv`, `-vvv`)? → Core Team → Future enhancement backlog

## External References (optional)

<!-- External standards, specifications, articles, or documentation only -->

- [Rust env_logger](https://docs.rs/env_logger) - Logging level configuration patterns

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.

- Tasks:
  - [T-h9q7d-log-tuning](../../tasks/T-h9q7d-log-tuning/README.md)

<!--lint enable remark-validate-links -->
