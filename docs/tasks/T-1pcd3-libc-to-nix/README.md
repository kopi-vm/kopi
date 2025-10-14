# T-1pcd3 libc to nix Usage Audit Task

## Metadata

- Type: Task
- Status: Cancelled
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Associated Design Document:
  - N/A – Design document not created (migration suspended)
- Associated Plan Document:
  - N/A – Plan document not created (migration suspended)

## Summary

Catalogue every direct `libc` invocation in Kopi, evaluate replacements with `nix`, and document whether migration delivers a safety benefit. Work concluded with a decision to suspend migration because five required filesystem constants are absent from `nix` 0.29.

## Scope

- In scope:
  - Audit all `libc` usages across the codebase.
  - Assess `nix` coverage, recommending keep vs. replace actions.
  - Route findings and open questions to analysis AN-i9cma.
- Out of scope:
  - Implement replacement code paths.
  - Modify `filesystem.rs` or update dependencies.

## Success Metrics

- Inventory coverage: Document every direct `libc` usage with a recommended disposition.
- Decision record: Capture the suspension rationale in AN-i9cma and circulate to maintainers.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
