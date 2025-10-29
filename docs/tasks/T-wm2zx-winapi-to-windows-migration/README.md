# T-wm2zx Winapi to Windows Crate Migration Task

## Metadata

- Type: Task
- Status: Cancelled
  <!-- Draft: Under discussion | In Progress: Actively working | Complete: Code complete | Cancelled: Work intentionally halted -->

## Links

- Related Analyses:
  - N/A – Pending discovery
- Related Requirements:
  - [FR-rxelv-file-in-use-detection](../../requirements/FR-rxelv-file-in-use-detection.md)
- Related ADRs:
  - N/A – None identified
- Associated Design Document:
  - N/A – Design not started
- Associated Plan Document:
  - N/A – Plan not started

## Summary

The proposed migration to replace `winapi` with the `windows` crate is cancelled. The current dependency graph shows multiple direct and transitive paths to `winapi`, and key crates we rely on today still require it:

```
cargo tree -i winapi --target all
winapi v0.3.9
├── crossterm v0.28.1
│   └── comfy-table v7.1.4
│       └── kopi v0.1.4 (/workspaces/kopi-workspace/first)
├── crossterm_winapi v0.9.1
│   └── crossterm v0.28.1 (*)
├── kopi v0.1.4 (/workspaces/kopi-workspace/first)
└── ntapi v0.4.1
    └── sysinfo v0.31.4
        └── kopi v0.1.4 (/workspaces/kopi-workspace/first)
```

We cannot drop `comfy-table` or `sysinfo` because they provide essential table rendering and system insight features for Kopi. Even if Kopi declared only the `windows` crate as a direct dependency, these transitive `winapi` dependencies would remain, leading to duplicated linker modules and an unjustified increase in the Windows binary size.

## Scope

- In scope:
  - Catalogue current `winapi` usages across the codebase and identify replacement APIs in the `windows` crate.
  - Define a phased rollout strategy that keeps existing Windows functionality stable during the transition.
  - Document required build configuration updates and potential feature flag implications.
- Out of scope:
  - Immediate implementation of the migration (will follow after task approval).
  - Changes to non-Windows platform bindings.

## Success Metrics

- Migration roadmap documented and approved with clear phase boundaries.
- Risk register and compatibility notes published for downstream maintainers before implementation starts.

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](../../templates/README.md#task-template-taskmd) in the templates README.
