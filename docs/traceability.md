# Traceability Matrix

## Overview
Central mapping of the complete development workflow: analysis → requirements → tasks → tests, including ADRs for the Kopi project.

## Analysis → Requirements Mapping

| Analysis Document | Status | Discovered Requirements | Date Completed |
|------------------|--------|------------------------|----------------|
| [cache-optimization.md](../analysis/cache-optimization.md) | Complete | FR-0001, FR-0002, NFR-0001 | YYYY-MM-DD |
| [javafx-support.md](../analysis/javafx-support.md) | Active | FR-DRAFT-003, NFR-DRAFT-002 | - |

## Requirements → Tasks → Tests Matrix

| Requirement ID | Title | Source Analysis | Status | Tasks | Tests | ADRs | Notes |
|---------------|-------|-----------------|--------|-------|-------|------|-------|
| FR-0001 | [Requirement Title] | cache-optimization | Proposed | [task-name] | [test names] | [ADR-###] | [Notes] |
| NFR-0001 | [Non-functional Requirement] | cache-optimization | Implemented | [task-name] | [test names] | [ADR-###] | [Notes] |

## Status Legend
- **Proposed**: Requirement defined but not yet implemented
- **In Progress**: Currently being implemented in one or more tasks
- **Implemented**: Code complete but not fully verified
- **Verified**: Implementation complete and all tests passing
- **Deprecated**: No longer applicable

## Task Status

| Task | Design | Plan | Status | Primary Requirements | Completion Date |
|------|--------|------|--------|---------------------|-----------------|
| [task-name] | [✓/✗] | [✓/✗] | [Status] | FR-####, NFR-#### | YYYY-MM-DD |

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