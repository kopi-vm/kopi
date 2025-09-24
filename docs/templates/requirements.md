# `[Requirement Title]`

## Metadata

- Type: Functional Requirement | Non-Functional Requirement
- Status: Proposed | Accepted | Implemented | Verified | Deprecated
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->

## Links

<!-- Internal project artifacts only. Replace or remove bullets as appropriate. -->

- Related Analyses:
  - [AN-<id>-<topic>](../analysis/AN-<id>-<topic>.md)
- Prerequisite Requirements:
  - [FR-<id>-<name>](../requirements/FR-<id>-<name>.md)
  - [NFR-<id>-<name>](../requirements/NFR-<id>-<name>.md)
- Dependent Requirements:
  - [FR-<id>-<name>](../requirements/FR-<id>-<name>.md)
- Related ADRs:
  - [ADR-<id>-<title>](../adr/ADR-<id>-<title>.md)
- Related Tasks:
  - [T-<id>-<task>](../tasks/T-<id>-<task>/README.md)

## Requirement Statement

`[Clear, concise, unambiguous statement of what is required. One requirement per document. Be specific and measurable.]`

Examples:

- FR: "The system shall provide a command to list all installed JDK versions"
- NFR: "JDK installation shall complete within 60 seconds for versions under 500MB"

## Rationale

`[Why this requirement exists. What problem does it solve? What value does it provide?]`

## User Story (if applicable)

`[For functional requirements]`
As a `[persona]`, I want `[capability]`, so that `[benefit]`.

`[For non-functional requirements]`
The system shall `[constraint/quality attribute]` to ensure `[benefit/goal]`.

## Acceptance Criteria

`[Specific, measurable, testable conditions that must be met]`

- [ ] `[Criterion 1 - be specific and testable]`
- [ ] `[Criterion 2 - include metrics where applicable]`
- [ ] `[Criterion 3 - reference test names when known]`
- [ ] `[Criterion 4 - platform-specific behavior if needed]`

## Technical Details (if applicable)

### Functional Requirement Details

`[For FRs: Detailed behavior, inputs/outputs, error conditions]`

### Non-Functional Requirement Details

`[For NFRs: Specific constraints, thresholds, standards]`

- Performance: `[Latency/throughput targets]`
- Security: `[Security requirements, standards]`
- Reliability: `[Availability, retry behavior]`
- Compatibility: `[Platform-specific requirements]`
- Usability: `[UX requirements, message standards]`

## Platform Considerations

### Unix

`[Unix-specific behavior or requirements]` | N/A – Platform agnostic

### Windows

`[Windows-specific behavior or requirements]` | N/A – Platform agnostic

### Cross-Platform

`[Behavior that must be consistent across platforms]` | N/A – Platform agnostic

## Risks & Mitigation

| Risk                 | Impact          | Likelihood      | Mitigation              | Validation                   |
| -------------------- | --------------- | --------------- | ----------------------- | ---------------------------- |
| `[Risk description]` | High/Medium/Low | High/Medium/Low | `[Mitigation strategy]` | `[How to verify mitigation]` |

## Implementation Notes

`[Any guidance for implementers. This is NOT a design document but can include:]`

- Preferred approaches or patterns to follow
- Known pitfalls to avoid
- Related code areas or modules
- Suggested libraries or tools

## External References

<!-- Only external resources. Internal documents go in Links section -->

- [External specification or standard](https://example.com) - Description | N/A – No external references

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](README.md#individual-requirement-template-requirementsmd) in the templates README.

- Tasks:
  - [T-<id>-<task>](../tasks/T-<id>-<task>/README.md)

<!--lint enable remark-validate-links -->
