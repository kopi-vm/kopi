# `libc` to `nix` Usage Audit Task

## Metadata

- Type: Task
- Status: Proposed

## Links

- Related Analyses:
  - N/A – No supporting analysis yet
- Related Requirements:
  - N/A – No requirements defined yet
- Related ADRs:
  - N/A – No ADRs linked yet
- Associated Plan Document:
  - N/A – Plan document not started
- Associated Design Document:
  - N/A – Design document not started

## Summary

Audit every direct `libc` invocation in the project and determine where replacing it with `nix` abstractions would reduce unsafe code while preserving behaviour.

## Scope

- In scope: Catalogue `libc` usage, evaluate `nix` equivalents, recommend migrations or justifications for remaining direct bindings
- Out of scope: Implementing the replacements (will follow in separate work)

## Success Metrics

- Coverage: 100% of current direct `libc` calls reviewed and documented
- Recommendations: Clear keep-or-replace decision recorded for each call site

## Detailed Plan

- Inventory existing direct `libc` imports and call sites across the codebase
- For each site, assess `nix` support, safety benefits, and required refactors
- Produce migration recommendations, grouping related changes where possible

## Notes

- Task created in response to desire to minimise `unsafe` usage by preferring `nix` wrappers when practical.
