# `libc` to `nix` Migration Analysis

## Metadata

- Type: Analysis
- Status: Complete
- Decision: Migration suspended due to incomplete `nix` constant coverage

## Links

- Related Analyses:
  - N/A
- Related Requirements:
  - N/A
- Related ADRs:
  - N/A
- Related Tasks:
  - [T-1pcd3-libc-to-nix](../tasks/T-1pcd3-libc-to-nix/README.md)

## Executive Summary

This analysis documents all direct `libc` crate usage in the Kopi codebase and evaluates opportunities to replace it with safer `nix` crate abstractions. The audit found minimal `libc` usage: only type declarations (`libc::c_long`) for filesystem magic number constants in the filesystem inspection module. Direct migration to `nix` types is partially feasible but blocked by incomplete constant coverage in `nix` 0.29.

**Key Finding**: The codebase already leverages `nix` for all unsafe operations. The remaining `libc` usage is limited to type annotations that could be replaced with `nix::sys::statfs::FsType`, but some filesystem magic constants (ZFS, CIFS/SMB2, VFAT, EXFAT) are not available in the current `nix` version.

## Problem Space

### Current State

**Dependency Status**:

- `libc = "0.2"` declared as Unix-only dependency in `Cargo.toml` (line 72)
- `nix = { version = "0.29", features = ["fs"] }` already in use (line 73)

**Direct `libc` Usage Locations**:

All usage is concentrated in `src/platform/filesystem.rs`:

1. **Type Cast** (line 170):

   ```rust
   let raw = fs_type.0 as libc::c_long;
   ```

   Casts `nix::sys::statfs::FsType` inner field to `libc::c_long` for comparison.

2. **Function Parameters** (lines 187, 249):

   ```rust
   fn classify_unix_magic(raw: libc::c_long) -> Option<FilesystemInfo>
   fn classify_by_name(name: &str, fallback_raw: libc::c_long) -> FilesystemInfo
   ```

3. **Constant Definitions** (lines 271-293):
   ```rust
   #[cfg(target_os = "linux")]
   const EXT4_SUPER_MAGIC: libc::c_long = 0xEF53;
   const XFS_SUPER_MAGIC: libc::c_long = 0x5846_5342;
   const BTRFS_SUPER_MAGIC: libc::c_long = 0x9123_683E;
   const TMPFS_MAGIC: libc::c_long = 0x0102_1994;
   const OVERLAYFS_SUPER_MAGIC: libc::c_long = 0x794C_7630;
   const ZFS_SUPER_MAGIC: libc::c_long = 0x2FC1_2FC1;
   const CIFS_MAGIC_NUMBER: libc::c_long = 0xFF53_4D42;
   const SMB2_MAGIC_NUMBER: libc::c_long = 0xFE53_4D42;
   const NFS_SUPER_MAGIC: libc::c_long = 0x0000_6969;
   const MSDOS_SUPER_MAGIC: libc::c_long = 0x0000_4D44;
   const VFAT_SUPER_MAGIC: libc::c_long = 0x0000_5646;
   const EXFAT_SUPER_MAGIC: libc::c_long = 0x2011_BAB0;
   ```

**Non-`libc` Matches** (false positives):

- Variable/field names containing "libc": `libc_variant`, `lib_c_type`, `get_foojay_libc_type()` - these are domain names referring to JDK libc compatibility, not the Rust `libc` crate.

### Desired State

Replace all `libc::c_long` type references with `nix::sys::statfs::FsType` to:

1. Eliminate direct `libc` dependency where feasible
2. Use higher-level, safer type abstractions
3. Improve type safety by avoiding raw `c_long` comparisons

### Gap Analysis

**`nix` 0.29 Coverage Analysis**:

Constants **available** in `nix::sys::statfs`:

- ✅ `EXT4_SUPER_MAGIC`
- ✅ `XFS_SUPER_MAGIC`
- ✅ `BTRFS_SUPER_MAGIC`
- ✅ `TMPFS_MAGIC`
- ✅ `OVERLAYFS_SUPER_MAGIC`
- ✅ `NFS_SUPER_MAGIC`
- ✅ `MSDOS_SUPER_MAGIC`
- ✅ `SMB_SUPER_MAGIC`

Constants **missing** in `nix::sys::statfs`:

- ❌ `ZFS_SUPER_MAGIC`
- ❌ `CIFS_MAGIC_NUMBER`
- ❌ `SMB2_MAGIC_NUMBER`
- ❌ `VFAT_SUPER_MAGIC`
- ❌ `EXFAT_SUPER_MAGIC`

**Technical Blockers**:

- Cannot fully migrate while 5 constants are unavailable in `nix`
- Would require either:
  1. Continuing to define missing constants manually using `libc::c_long`
  2. Defining custom `FsType` constants wrapping the raw values
  3. Waiting for upstream `nix` to add these constants

## Stakeholder Analysis

| Stakeholder       | Interest/Need                | Impact | Priority |
| ----------------- | ---------------------------- | ------ | -------- |
| Maintainers       | Reduce unsafe code surface   | Medium | P1       |
| Security auditors | Minimize FFI boundary points | Low    | P2       |
| Users             | None (internal change)       | Low    | P2       |

## Research & Discovery

### Technical Investigation

**`nix` Crate Documentation Review** (nix 0.29):

- `nix::sys::statfs::FsType` is a newtype wrapper: `pub struct FsType(pub __fsword_t)`
- `__fsword_t` is a `libc` type, so `nix` itself depends on `libc`
- Magic constants are defined as `pub const <NAME>: FsType = FsType(libc::<NAME>);`
- Coverage is incomplete for less common filesystems

**Current Code Safety Analysis**:

- No `unsafe` blocks in filesystem.rs related to `libc` usage
- All actual system calls go through safe `nix::sys::statfs::statfs()` wrapper
- `libc` usage is confined to type-level annotations

### Competitive Analysis

Similar tools (rustup, volta, mise) generally:

- Use `nix` for Unix system calls
- May use `libc` directly only where `nix` doesn't provide coverage
- Accept mixed `libc`/`nix` usage as pragmatic

## Discovered Requirements

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: Minimize direct `libc` usage where safe alternatives exist
  - Category: Security
  - Rationale: Reducing FFI surface area improves safety and maintainability
  - Target: Use `nix` abstractions for all operations with available coverage

## Design Considerations

### Technical Constraints

1. **Incomplete `nix` Coverage**: Five filesystem magic constants are unavailable
2. **Type Compatibility**: `FsType` wraps a `libc` type internally, so some `libc` dependency remains transitive
3. **Comparison Semantics**: Must maintain exact equality checking behavior

### Potential Approaches

#### Option A: Partial Migration (Use `nix` Constants Where Available)

**Description**: Replace constants available in `nix`, keep `libc::c_long` for missing ones.

```rust
use nix::sys::statfs::{
    EXT4_SUPER_MAGIC, XFS_SUPER_MAGIC, BTRFS_SUPER_MAGIC,
    TMPFS_MAGIC, OVERLAYFS_SUPER_MAGIC, NFS_SUPER_MAGIC,
    MSDOS_SUPER_MAGIC, SMB_SUPER_MAGIC,
};

// Still need libc for missing constants
const ZFS_SUPER_MAGIC: libc::c_long = 0x2FC1_2FC1;
const CIFS_MAGIC_NUMBER: libc::c_long = 0xFF53_4D42;
// ...

fn classify_unix_magic(fs_type: nix::sys::statfs::FsType) -> Option<FilesystemInfo> {
    match fs_type {
        EXT4_SUPER_MAGIC => Some(...),
        XFS_SUPER_MAGIC => Some(...),
        // Cast for missing constants
        ft if ft.0 as libc::c_long == ZFS_SUPER_MAGIC => Some(...),
        _ => None,
    }
}
```

- **Pros**: Leverages `nix` where available, improves type safety for 8/13 constants
- **Cons**: Mixed approach with inconsistent types, requires casting
- **Effort**: Medium

#### Option B: Define Custom `FsType` Wrappers

**Description**: Manually create `FsType` values for missing constants.

```rust
use nix::sys::statfs::FsType;

const ZFS_SUPER_MAGIC: FsType = FsType(0x2FC1_2FC1);
const CIFS_MAGIC_NUMBER: FsType = FsType(0xFF53_4D42);
// ...

fn classify_unix_magic(fs_type: FsType) -> Option<FilesystemInfo> {
    match fs_type {
        EXT4_SUPER_MAGIC => Some(...),
        ZFS_SUPER_MAGIC => Some(...),  // Custom constant
        _ => None,
    }
}
```

- **Pros**: Uniform `FsType` usage, cleaner API, no `libc` imports needed
- **Cons**: Relies on `FsType` internal structure (public field), may break on `nix` updates
- **Effort**: Low

#### Option C: Status Quo (Keep Current `libc::c_long` Approach)

**Description**: No changes; continue using `libc::c_long` for all constants.

- **Pros**: No migration risk, works today, minimal effort
- **Cons**: Doesn't reduce `libc` surface area, misses type safety benefits
- **Effort**: None

#### Option D: Contribute Missing Constants to `nix` Upstream

**Description**: Submit PR to `nix` to add missing constants, then migrate.

- **Pros**: Benefits entire ecosystem, cleanest long-term solution
- **Cons**: Depends on upstream acceptance timeline, no immediate benefit
- **Effort**: Medium-High (requires upstream coordination)

### Architecture Impact

No ADRs required - this is an internal implementation detail with no user-facing impact.

## Risk Assessment

| Risk                                          | Probability | Impact | Mitigation Strategy                          |
| --------------------------------------------- | ----------- | ------ | -------------------------------------------- |
| `nix` internal changes break custom constants | Low         | Medium | Pin `nix` version, monitor breaking changes  |
| Missed filesystem type during migration       | Low         | Medium | Comprehensive test coverage on Linux         |
| Performance regression from type changes      | Very Low    | Low    | Zero-cost abstractions, benchmarks unchanged |

## Open Questions

- [x] Does `nix` 0.29 provide all needed filesystem constants? → **No, 5 are missing**
- [x] Is `FsType.0` field guaranteed stable? → **Yes, it's public but undocumented stability**
- [ ] Should we contribute missing constants upstream to `nix`? → Decision needed

## Recommendations

### Decision: Migration Suspended

**Status**: The proposed migration to `nix` types has been suspended based on the coverage gap identified in this analysis.

**Rationale**:

- 5 of 13 filesystem magic constants are unavailable in `nix` 0.29
- The current `libc::c_long` approach involves no unsafe code
- Migration would require either custom wrappers (fragile) or incomplete coverage
- The benefit of migration is limited given the safe, type-annotation-only usage

**Original Recommendation** (Not Implemented):

~~**Recommendation: Option B (Define Custom `FsType` Wrappers) + Option D (Contribute Upstream)**~~

1. **Short-term (Current Release)**:
   - Implement Option B to eliminate `libc::c_long` usage
   - Define missing constants as `FsType` wrappers
   - Remove direct `libc` dependency (it remains transitive via `nix`)

2. **Long-term (Future Contribution)**:
   - Submit PR to `nix` project adding: `ZFS_SUPER_MAGIC`, `CIFS_MAGIC_NUMBER`, `SMB2_MAGIC_NUMBER`, `VFAT_SUPER_MAGIC`, `EXFAT_SUPER_MAGIC`
   - Once merged and in a released `nix` version, remove custom wrappers

### Justification for Suspension

- Current code is safe and maintainable
- Coverage gap blocks clean migration
- Effort-to-benefit ratio does not justify workarounds
- Can revisit when `nix` upstream adds missing constants
- No security or safety impact from current implementation

### Next Steps

1. [x] Complete this analysis
2. [x] Document suspension decision in task T-1pcd3
3. [ ] ~~Create design document detailing migration approach~~ (Suspended)
4. [ ] ~~Create plan document with implementation steps~~ (Suspended)
5. [ ] ~~Implement Option B migration~~ (Suspended)
6. [ ] (Future) Monitor `nix` releases for addition of missing constants
7. [ ] (Future) Revisit migration if coverage gap is resolved

### Out of Scope

- Modifying `nix` crate functionality (beyond potential upstream contribution)
- Changing filesystem detection logic or supported filesystem types
- Performance optimization of filesystem detection
- Supporting additional operating systems beyond current Linux/macOS/Windows coverage

## Appendix

### References

- [nix crate documentation](https://docs.rs/nix/0.29.0/nix/)
- [nix::sys::statfs module](https://docs.rs/nix/0.29.0/nix/sys/statfs/)
- [nix source code](https://github.com/nix-rust/nix/blob/master/src/sys/statfs.rs)
- [Linux filesystem magic numbers](https://github.com/torvalds/linux/blob/master/include/uapi/linux/magic.h)

### Raw Data

**Complete `libc` Usage Inventory**:

```
src/platform/filesystem.rs:170:    let raw = fs_type.0 as libc::c_long;
src/platform/filesystem.rs:187:fn classify_unix_magic(raw: libc::c_long) -> Option<FilesystemInfo> {
src/platform/filesystem.rs:249:fn classify_by_name(name: &str, fallback_raw: libc::c_long) -> FilesystemInfo {
src/platform/filesystem.rs:271:const EXT4_SUPER_MAGIC: libc::c_long = 0xEF53;
src/platform/filesystem.rs:273:const XFS_SUPER_MAGIC: libc::c_long = 0x5846_5342;
src/platform/filesystem.rs:275:const BTRFS_SUPER_MAGIC: libc::c_long = 0x9123_683E;
src/platform/filesystem.rs:277:const TMPFS_MAGIC: libc::c_long = 0x0102_1994;
src/platform/filesystem.rs:279:const OVERLAYFS_SUPER_MAGIC: libc::c_long = 0x794C_7630;
src/platform/filesystem.rs:281:const ZFS_SUPER_MAGIC: libc::c_long = 0x2FC1_2FC1;
src/platform/filesystem.rs:283:const CIFS_MAGIC_NUMBER: libc::c_long = 0xFF53_4D42;
src/platform/filesystem.rs:285:const SMB2_MAGIC_NUMBER: libc::c_long = 0xFE53_4D42;
src/platform/filesystem.rs:287:const NFS_SUPER_MAGIC: libc::c_long = 0x0000_6969;
src/platform/filesystem.rs:289:const MSDOS_SUPER_MAGIC: libc::c_long = 0x0000_4D44;
src/platform/filesystem.rs:291:const VFAT_SUPER_MAGIC: libc::c_long = 0x0000_5646;
src/platform/filesystem.rs:293:const EXFAT_SUPER_MAGIC: libc::c_long = 0x2011_BAB0;
```

**Dependency Declaration**:

```toml
# Cargo.toml:72-73
[target.'cfg(unix)'.dependencies]
libc = "0.2"
nix = { version = "0.29", default-features = false, features = ["fs"] }
```
