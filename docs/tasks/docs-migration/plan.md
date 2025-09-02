# ADR Archive Migration Implementation Plan

## Metadata
- Type: Implementation Plan
- Owner: Development Team
- Reviewers: Project Maintainers
- Status: Phase 1 Completed
  <!-- Not Started: Planning complete, awaiting start | Phase X In Progress: Actively working | Blocked: External dependency | Under Review: Implementation complete | Completed: All phases done and verified -->
- Date Created: 2025-09-02

## Links
<!-- Internal project artifacts only. For external resources, see External References section -->
- Requirements: N/A – No formal requirements for documentation restructuring
- Design: N/A – Straightforward file archival operation
- Related ADRs: N/A – This is a documentation organization update
- Issue: N/A – Internal documentation improvement
- PR: N/A – To be created during implementation

## Overview

This plan migrates existing ADR files and task directories to archive structures to better organize documentation. All 19 existing ADR files will be moved to `docs/adr/archive/` and all 14 existing task directories will be moved to `docs/tasks/archive/` to distinguish them from future documents that will follow new naming conventions. This maintains historical documentation while clearing the main directories for future documents with standardized naming.

## Success Metrics
- [ ] All 19 ADR files moved to docs/adr/archive/
- [ ] All 14 existing task directories moved to docs/tasks/archive/
- [ ] All internal links updated and functional
- [ ] Git history preserved for all moved files and directories
- [ ] Documentation templates updated with new naming conventions
- [ ] AGENTS.md updated with new ADR format
- [ ] No broken links in any documentation

## Scope
- Goal: 
  - Archive existing ADR files to `docs/adr/archive/` directory
  - Archive existing task directories to `docs/tasks/archive/` directory
  - Introduce new naming conventions for future documents
- Non-Goals: 
  - Renaming existing ADR files or task directories (keeping original names)
  - Changing FR/NFR document types (already compliant)
  - Modifying document content
  - Moving docs-migration task (current active task)
- Assumptions: Git mv will preserve file history
- Constraints: Minimize disruption to ongoing work

## Plan Summary
- Phases: 3 phases covering archive creation, file migration, and link updates
- Timeline: Single session implementation (estimated 1-2 hours)

---

## Phase 1: Archive Setup and Naming Convention Introduction

### Goal
- Create archive directory structure and introduce new naming conventions for future documents

### Inputs
- Documentation:
  - `/docs/templates/README.md` – Template overview and guidelines (to be updated with new naming conventions)
  - `/AGENTS.md` – Project process documentation (to be updated with new ADR format)
  - `/docs/adr/MIGRATION.md` – To be removed as obsolete
- Directories to create:
  - `/docs/adr/archive/` – Archive location for existing ADRs
  - `/docs/tasks/archive/` – Archive location for existing task directories

### Tasks
- [x] **Create archive directories**
  - [x] Create `/docs/adr/archive/` directory
  - [x] Create `/docs/tasks/archive/` directory
  - [x] Add README.md to ADR archive directory explaining its purpose
  - [x] Add README.md to tasks archive directory explaining its purpose
- [x] **Update documentation with new naming conventions**
  - [x] Update docs/templates/README.md Document Organization table with new naming conventions
    - [x] Add new naming format: ADR-####-<title> for future ADRs
    - [x] Add new naming format: T-####-<name> for task directories
    - [x] Add new naming format: AN-####-<title> for analysis documents
    - [x] Note that FR/NFR formats remain unchanged (already compliant)
    - [x] Update ADR location to note both main directory (for new) and archive (for existing)
  - [x] Update AGENTS.md
    - [x] Update ADR reference format from `###-<title>` to `ADR-####-<title>` for future ADRs
    - [x] Add note about archived ADRs location
  - [x] Remove docs/adr/MIGRATION.md as obsolete

### Deliverables
- Archive directory structure created
- New naming conventions documented for future documents
- Templates and AGENTS.md updated with new formats
- Documentation updated to reflect both archive and new conventions

### Verification
```bash
# Verify directories exist
ls -la docs/adr/archive/
ls -la docs/tasks/archive/
# Verify MIGRATION.md removed
test ! -f docs/adr/MIGRATION.md && echo "MIGRATION.md successfully removed"
# Verify new naming conventions documented
grep -E "ADR-[0-9]{4}" docs/templates/README.md
grep -E "T-[0-9]{4}" docs/templates/README.md
# Verify AGENTS.md updated
grep -E "ADR-[0-9]{4}" AGENTS.md
```

### Acceptance Criteria (Phase Gate)
- Archive directories exist and are ready for files (both ADR and tasks)
- New naming conventions documented in docs/templates/README.md
- AGENTS.md updated with new ADR format
- Documentation reflects both archive structures and new naming conventions

### Rollback/Fallback
- Remove archive directory if issues arise

---

## Phase 2: ADR and Task Migration to Archives

### Goal
- Move all 19 ADR files to ADR archive directory
- Move all 14 existing task directories to task archive directory

### Inputs
- Dependencies:
  - Phase 1: Archive directories created
- Files to move (19 ADR files):
  - `/docs/adr/001-kopi-command-structure.md`
  - `/docs/adr/002-serialization-format-for-metadata-storage.md`
  - `/docs/adr/003-jdk-storage-format.md`
  - `/docs/adr/004-error-handling-strategy.md`
  - `/docs/adr/005-web-api-mocking-strategy.md`
  - `/docs/adr/006-progress-indicator-strategy.md`
  - `/docs/adr/007-default-jdk-distribution-selection.md`
  - `/docs/adr/008-platform-compatibility-strategy.md`
  - `/docs/adr/009-logging-strategy.md`
  - `/docs/adr/010-api-version-fallback-strategy.md`
  - `/docs/adr/011-jre-support-strategy.md`
  - `/docs/adr/012-build-and-test-performance-optimization.md`
  - `/docs/adr/013-binary-switching-approaches.md`
  - `/docs/adr/014-configuration-and-version-file-formats.md`
  - `/docs/adr/015-version-manager-migration-support.md`
  - `/docs/adr/016-flexible-version-format.md`
  - `/docs/adr/017-jdk-release-metadata-sources.md`
  - `/docs/adr/018-macos-jdk-bundle-structure-handling.md`
  - `/docs/adr/019-version-switching-command-design.md`
- Task directories to move (14 existing tasks):
  - `/docs/tasks/ap-bundle/`
  - `/docs/tasks/doctor/`
  - `/docs/tasks/env/`
  - `/docs/tasks/indicator/`
  - `/docs/tasks/install/`
  - `/docs/tasks/lock/`
  - `/docs/tasks/metadata/`
  - `/docs/tasks/search/`
  - `/docs/tasks/shims/`
  - `/docs/tasks/switch/`
  - `/docs/tasks/uninstall/`
  - `/docs/tasks/version/`
  - `/docs/tasks/which/`
  - Note: `/docs/tasks/docs-migration/` stays (this current task)

### Tasks
- [ ] **Move ADR files using git mv**
  - [ ] Move all 19 ADR files to docs/adr/archive/
  - [ ] Preserve original file names (no renaming)
  - [ ] Verify git history is maintained
- [ ] **Move task directories using git mv**
  - [ ] Move all 14 existing task directories to docs/tasks/archive/
  - [ ] Preserve original directory names (no renaming)
  - [ ] Keep docs-migration in place (current active task)

### Deliverables
- All ADR files moved to ADR archive with preserved git history
- All existing task directories moved to task archive with preserved git history

### Verification
```bash
# Verify all ADR files moved
ls docs/adr/archive/*.md | wc -l  # Should output: 19
# Verify no ADR files remain in main directory
ls docs/adr/*.md 2>/dev/null | wc -l  # Should output: 0
# Verify all task directories moved
ls -d docs/tasks/archive/*/ | wc -l  # Should output: 14
# Verify only docs-migration remains in main tasks directory
ls -d docs/tasks/*/ | grep -v archive | wc -l  # Should output: 1
# Verify git history preserved
git log --follow docs/adr/archive/001-kopi-command-structure.md
git log --follow docs/tasks/archive/install/
```

### Acceptance Criteria (Phase Gate)
- All 19 ADR files successfully moved to ADR archive
- All 14 task directories successfully moved to task archive
- Git history preserved for all files and directories
- Main ADR directory clear of old files
- Main tasks directory contains only docs-migration (current task)

### Rollback/Fallback
- Git mv files back to original location if issues arise

---

## Phase 3: Cross-Reference Updates

### Goal
- Update all internal documentation links to point to archived ADR locations

### Inputs
- Dependencies:
  - Phase 2: ADR files moved to archive
- Files potentially containing ADR references:
  - All .md files in docs/
  - README.md files throughout the project
  - Source code comments (if any)

### Tasks
- [ ] **Update ADR references in documentation**
  - [ ] Search for ADR references in all .md files
  - [ ] Update paths from `docs/adr/` to `docs/adr/archive/`
  - [ ] Update docs/templates/README.md example links
  - [ ] Update any ADR cross-references within archived ADR files themselves
- [ ] **Update source code references (if any)**
  - [ ] Search for ADR references in Rust source comments
  - [ ] Update any found references to archive path

### Deliverables
- All documentation with updated ADR archive links
- No broken internal references

### Verification
```bash
# Check for old ADR patterns (should only be in archive)
grep -r "docs/adr/[0-9][0-9][0-9]-" docs/ --include="*.md" | grep -v archive
# Should return no results outside archive

# Verify archive references exist
grep -r "docs/adr/archive/[0-9][0-9][0-9]-" docs/ --include="*.md"
# Should return updated references
```

### Acceptance Criteria (Phase Gate)
- No broken ADR links outside archive directory
- All links to ADRs point to archive location

### Rollback/Fallback
- Use git diff to identify and revert link changes if needed

## Testing Strategy

### Documentation Testing
- Manual review of all changed links
- Automated grep patterns to find broken references
- Git log verification for history preservation

### Integration Testing
- Verify documentation renders correctly in GitHub
- Check that IDEs can follow the archive links
- Ensure any documentation generation tools still work

---

## Dependencies

### External Tools
- `git` – For mv operations and history preservation
- `grep` – For searching and verification
- Standard Unix tools (ls, wc, find)

### Internal Modules
- No code module dependencies (documentation only)

---

## Risks & Mitigations

1. Risk: Breaking external links from issues/PRs
   - Mitigation: Archive maintains file names, only path changes
   - Validation: Search GitHub issues/PRs for ADR links
   - Fallback: Could symlink from old location if critical

2. Risk: Git history loss during move
   - Mitigation: Use git mv exclusively (not delete + add)
   - Validation: Test with one file first
   - Fallback: Restore from backup branch

3. Risk: Missing some references during update
   - Mitigation: Multiple search patterns and manual review
   - Validation: Automated grep verification
   - Fallback: Fix incrementally as found

---

## Documentation & Change Management

### Documentation Updates
- This plan itself documents the migration
- README files will reflect new conventions
- Consider adding a CHANGELOG entry

### Communication
- Note in next PR that documentation naming has been standardized
- Update any contributor guidelines if they exist

---

## Implementation Guidelines

### File Operations
- Use `git mv` for all moves to preserve history
- Batch operations where possible for efficiency
- Verify each phase before proceeding

### Archive Standards
- Maintain original file names in archive
- Preserve directory structure if needed
- Keep archive README up to date

---

## Definition of Done

- [ ] Archive directories created at docs/adr/archive/ and docs/tasks/archive/
- [ ] Archive README.md files explaining purpose in both archives
- [ ] All 19 ADR files moved to ADR archive directory
- [ ] All 14 existing task directories moved to task archive directory
- [ ] All internal documentation links updated to archive paths
- [ ] docs/adr/MIGRATION.md removed
- [ ] New naming conventions documented in templates
- [ ] AGENTS.md updated with new ADR format
- [ ] No broken links in any documentation
- [ ] Git history preserved for all moved files and directories
- [ ] Verification scripts run successfully
- [ ] Migration documented in this plan

---

## Status Tracking

- Not Started: Current state, plan complete
- Phase 1 In Progress: Archive setup
- Phase 2 In Progress: ADR migration to archive
- Phase 3 In Progress: Link updates
- Completed: All phases done and verified

---

## Open Questions

- None identified – straightforward file archival operation

---

## Notes

This plan archives existing ADRs and task directories to maintain historical documentation while clearing the main directories for future documents. After this migration:
- Existing ADRs will be in: `docs/adr/archive/`
- Existing task directories will be in: `docs/tasks/archive/`
- New ADRs will use: `ADR-####-<title>.md` in the main `docs/adr/` directory
- New tasks will use: `T-####-<name>/` in the main `docs/tasks/` directory with `design.md` and `plan.md` files
- The archives preserve the original names and git history
- docs-migration task remains in main directory as the current active task

---

## Template Usage

This plan follows the existing template from [`docs/templates/plan.md`](../../templates/plan.md). Future plans will be created in task directories following the new `T-####-<name>/` convention.