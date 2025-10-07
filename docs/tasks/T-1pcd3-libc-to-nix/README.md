# `libc` to `nix` Usage Audit Task

## Metadata

- Type: Task
- Status: Suspended

## Links

- Related Analyses:
  - [AN-i9cma-libc-to-nix-migration](../../analysis/AN-i9cma-libc-to-nix-migration.md)
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

## Results

The audit is complete. See [AN-i9cma-libc-to-nix-migration](../../analysis/AN-i9cma-libc-to-nix-migration.md) for the full analysis.

**Summary**:

- All direct `libc` usage is concentrated in `src/platform/filesystem.rs`
- Only type declarations (`libc::c_long`) are used, no unsafe operations
- 8 of 13 filesystem magic constants are available in `nix` 0.29
- Coverage gap identified: 5 constants missing from `nix` 0.29

## Suspension Decision

**Status**: Migration work suspended based on analysis findings.

**Rationale**:

The analysis revealed that 5 of 13 required filesystem magic constants are unavailable in `nix` 0.29:

- `ZFS_SUPER_MAGIC`
- `CIFS_MAGIC_NUMBER`
- `SMB2_MAGIC_NUMBER`
- `VFAT_SUPER_MAGIC`
- `EXFAT_SUPER_MAGIC`

**Key factors in suspension decision**:

1. **No safety benefit**: Current `libc` usage involves only type annotations, no unsafe code
2. **Incomplete migration**: Cannot achieve full `nix` migration due to missing constants
3. **Workaround fragility**: Custom `FsType` wrappers would rely on internal structure
4. **Low priority**: Current implementation is safe, maintainable, and working correctly
5. **Cost-benefit ratio**: Effort required does not justify limited benefits

**Future considerations**:

- Monitor `nix` crate releases for addition of missing constants
- Revisit migration decision if coverage gap is resolved
- Consider contributing missing constants to `nix` upstream if prioritized
