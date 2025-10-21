# AN-<id> Topic | Feature Title

## Metadata

- Type: Analysis
- Status: Draft | Complete | Cancelled | Archived
  <!-- Draft: Initial exploration | Complete: Ready for requirements | Cancelled: Work intentionally halted | Archived: Analysis concluded -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Analyses:
  - [AN-<id>-<topic>](../analysis/AN-<id>-<topic>.md)
- Related Requirements:
  - [FR-<id>-<name>](../requirements/FR-<id>-<name>.md)
  - [NFR-<id>-<name>](../requirements/NFR-<id>-<name>.md)
- Related ADRs:
  - [ADR-<id>-<title>](../adr/ADR-<id>-<title>.md)
- Related Tasks:
  - [T-<id>-<task>](../tasks/T-<id>-<task>/README.md)

## Executive Summary

`[1-2 paragraph summary of the analysis scope, key findings, and recommendations]`

## Problem Space

### Current State

`[What exists today? What are the pain points?]`

### Desired State

`[What do we want to achieve? What would success look like?]`

### Gap Analysis

`[What's the delta between current and desired state?]`

## Stakeholder Analysis

| Stakeholder    | Interest/Need      | Impact          | Priority |
| -------------- | ------------------ | --------------- | -------- |
| `[User group]` | `[What they need]` | High/Medium/Low | P0/P1/P2 |

## Research & Discovery

### User Feedback

`[Surveys, interviews, support tickets, forum discussions]`

### Competitive Analysis

`[How do similar tools handle this?]`

### Technical Investigation

`[POCs, benchmarks, API exploration, feasibility studies]`

### Data Analysis

`[Metrics, logs, usage patterns if available]`

## Discovered Requirements

> Capture potential requirements as solution-agnostic problem statements focused on the problem to solve rather than any specific implementation.

### Functional Requirements (Potential)

- [ ] **FR-DRAFT-1**: `[Requirement description]` → Will become FR-<id>
  - Rationale: `[Why this is needed]`
  - Acceptance Criteria: `[Measurable criteria]`

- [ ] **FR-DRAFT-2**: `[Requirement description]` → Will become FR-<id>
  - Rationale: `[Why this is needed]`
  - Acceptance Criteria: `[Measurable criteria]`

### Non-Functional Requirements (Potential)

- [ ] **NFR-DRAFT-1**: `[Constraint/quality attribute]` → Will become NFR-<id>
  - Category: Performance | Security | Usability | Reliability
  - Rationale: `[Why this constraint matters]`
  - Target: `[Specific measurable target]`

## Design Considerations

### Technical Constraints

`[Known limitations, platform requirements, dependencies]`

### Potential Approaches

1. **Option A**: `[Description]`
   - Pros: `[Benefits]`
   - Cons: `[Drawbacks]`
   - Effort: High/Medium/Low

2. **Option B**: `[Description]`
   - Pros: `[Benefits]`
   - Cons: `[Drawbacks]`
   - Effort: High/Medium/Low

### Architecture Impact

`[Will this require new ADRs? What decisions need to be made?]`

## Risk Assessment

| Risk                 | Probability     | Impact          | Mitigation Strategy |
| -------------------- | --------------- | --------------- | ------------------- |
| `[Risk description]` | High/Medium/Low | High/Medium/Low | `[How to address]`  |

## Open Questions

- [ ] `[Question that needs investigation]`
- [ ] `[Decision that needs to be made]` → Next step: `[Where to resolve (e.g., draft ADR via docs/adr/ADR-<id>-<title>.md, create requirements docs/requirements/FR-<id>-<capability>.md per TDL)]`
- [ ] `[Information that needs gathering]` → Method: `[How to obtain insight]`

<!-- Complex investigations should spin out into their own ADR or analysis document -->

## Recommendations

### Immediate Actions

1. `[What should be done right away]`
2. `[Quick wins or critical fixes]`

### Next Steps

1. [ ] Create formal requirements: FR-<id>, NFR-<id>
2. [ ] Draft ADR for: `[Architectural decision needed]`
3. [ ] Create task for: `[Implementation work]`
4. [ ] Further investigation: `[What needs more analysis]`

### Out of Scope

`[What we explicitly decided NOT to do and why]`

## Appendix

### References

`[External documents, standards, articles that informed this analysis]`

### Raw Data

`[Survey results, benchmark outputs, logs - anything that supports the analysis]`

---

## Template Usage

For detailed instructions and key principles, see [Template Usage Instructions](README.md#analysis-template-analysismd) in the templates README.
