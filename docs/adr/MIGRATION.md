# ADR Templates Migration Guide

This guide outlines how to align existing ADRs with the new templates while keeping changes pragmatic and low‑risk.

## Goals
- Preserve history and URLs of existing ADRs
- Ensure mandatory traceability going forward (Links section)
- Avoid churn; migrate opportunistically

## Scope
- Applies to ADRs under `docs/adr/` created prior to the consolidated templates.
- New ADRs must use `docs/templates/adr.md` or `docs/templates/adr-lite.md`.

## Policy
- Leave existing ADRs as‑is unless you are actively updating them.
- When touching an existing ADR, bring it up to baseline:
  - Add a `Links` section (Requirements/Design/Plan/Issue/PR) with `N/A – <reason>` where not applicable.
  - Normalize `Status` to one of: Proposed | Accepted | Rejected | Deprecated | Superseded.
  - Add `Supersedes`/`Superseded by` references if relevant.
  - Ensure headings and bullets follow repository language/style guidelines (see `CLAUDE.md`).

## Prioritization
1. High‑impact ADRs referenced by active work (top priority)
2. Recently modified ADRs (opportunistic)
3. Older ADRs (on demand)

## Non‑Goals
- Do not rewrite history or content unless correctness is affected.
- Do not renumber ADRs.

## Verification
- On PRs that modify ADRs, verify:
  - Links section present and filled (or `N/A – <reason>`)
  - Status value valid; exit codes and error context referenced if applicable
  - Markdown renders cleanly across viewers

## References
- Templates: `docs/templates/adr.md`, `docs/templates/adr-lite.md`
- Examples: `docs/templates/examples/`
- Language/style: `CLAUDE.md`

