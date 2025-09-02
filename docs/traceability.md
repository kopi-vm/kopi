# Traceability Matrix

## Overview
Central mapping of the complete development workflow: analysis → requirements → tasks → tests, including ADRs for the Kopi project.

## Analysis → Requirements Mapping

| Analysis Document | Status | Discovered Requirements | Date Completed |
|------------------|--------|------------------------|----------------|
| [AN-0001-concurrent-process-locking.md](analysis/AN-0001-concurrent-process-locking.md) | Complete | FR-0001, FR-0002, FR-0003, FR-0004, FR-0005, NFR-0001, NFR-0002, NFR-0003 | 2025-09-02 |

## Requirements → Tasks → Tests Matrix

| Requirement ID | Title | Source Analysis | Status | Tasks | Tests | ADRs | Notes |
|---------------|-------|-----------------|--------|-------|-------|------|-------|
| FR-0001 | Process-level locking for installation | AN-0001 | Proposed | - | - | ADR-0001 | P0 priority |
| FR-0002 | Process-level locking for uninstallation | AN-0001 | Proposed | - | - | ADR-0001 | P0 priority |
| FR-0003 | Process-level locking for cache operations | AN-0001 | Proposed | - | - | ADR-0001 | P0 priority |
| FR-0004 | Lock timeout and recovery mechanism | AN-0001 | Proposed | - | - | ADR-0001 | P0 priority |
| FR-0005 | User feedback for lock contention | AN-0001 | Proposed | - | - | ADR-0001 | P1 priority |
| NFR-0001 | Lock acquisition timeout limit | AN-0001 | Proposed | - | - | ADR-0001 | Performance requirement |
| NFR-0002 | Lock cleanup reliability | AN-0001 | Proposed | - | - | ADR-0001 | Reliability requirement |
| NFR-0003 | Cross-platform lock compatibility | AN-0001 | Proposed | - | - | ADR-0001 | Compatibility requirement |

## Status Legend
- **Proposed**: Requirement defined but not yet implemented
- **In Progress**: Currently being implemented in one or more tasks
- **Implemented**: Code complete but not fully verified
- **Verified**: Implementation complete and all tests passing
- **Deprecated**: No longer applicable

## Task Status

| Task | Design | Plan | Status | Primary Requirements | Completion Date |
|------|--------|------|--------|---------------------|-----------------|
| - | - | - | - | - | - |

## Links
- Analysis Directory: [`docs/analysis/`](../analysis/)
- Requirements Directory: [`docs/requirements/`](../requirements/)
- Tasks Directory: [`docs/tasks/`](../tasks/)
- ADRs Directory: [`docs/adr/`](../adr/)

## Maintenance Notes
- Update this matrix when:
  - New analysis documents are created or completed
  - Requirements are discovered from analysis
  - New requirements are formalized with FR/NFR IDs
  - New tasks are created
  - Requirements are linked to tasks
  - Tests are added that verify requirements
  - ADRs are created that affect requirements
- Archive completed analysis documents but keep references here
- Use git history to track changes to this document
- In PRs, reference specific rows that were updated