# T-1pcd3 libc to nix Usage Audit Task

## Metadata

- Type: Task
- Status: Cancelled
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Associated Plan Document:
  - N/A – Plan document not created (migration suspended)
- Associated Design Document:
  - N/A – Design document not created (migration suspended)

## Summary

Catalogue every direct `libc` invocation in Kopi, evaluate replacements with `nix`, and document whether migration delivers a safety benefit. Work concluded with a decision to suspend migration because five required filesystem constants are absent from `nix` 0.29.

## Scope

- In scope: Audit all `libc` usages, assess `nix` coverage, recommend keep vs. replace actions, and route findings to analysis AN-i9cma.
- Out of scope: Implementing replacements, modifying `filesystem.rs`, or updating dependencies.

## Success Metrics

- Complete inventory of direct `libc` usages with recommended disposition.
- Documented rationale for suspending migration captured in AN-i9cma and shared with maintainers.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
