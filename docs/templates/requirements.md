# [FR-<id>/NFR-<id>]: [Requirement Title]

## Metadata
- ID: [FR-<id> or NFR-<id>]
- Type: Functional Requirement | Non-Functional Requirement  
- Category: [e.g., Performance, Security, Usability, API, CLI, Platform]
- Priority: P0 (Critical) | P1 (High) | P2 (Medium) | P3 (Low)
- Owner: [Person or role]
- Reviewers: [Names/roles]
- Status: Proposed | Accepted | Implemented | Verified | Deprecated
  <!-- Proposed: Under discussion | Accepted: Approved for implementation | Implemented: Code complete | Verified: Tests passing | Deprecated: No longer applicable -->
- Date Created: YYYY-MM-DD
- Date Modified: YYYY-MM-DD

## Links
<!-- Internal project artifacts only. For external resources, see External References section -->
- Implemented by Tasks: [`task-name-1`](../../tasks/task-name-1/), [`task-name-2`](../../tasks/task-name-2/) | N/A – Not yet implemented
- Related Requirements: FR-<id>, NFR-<id> | N/A – Standalone requirement
- Related ADRs: [ADR-<id>](../../adr/ADR-<id>-title.md) | N/A – No related ADRs
- Tests: `test_name_fr_<id>`, `bench_name_nfr_<id>` | N/A – Not yet tested
- Issue: #XXX | N/A – <reason>
- PR: #XXX | N/A – <reason>

## Requirement Statement

[Clear, concise, unambiguous statement of what is required. One requirement per document. Be specific and measurable.]

Examples:
- FR: "The system shall provide a command to list all installed JDK versions"
- NFR: "JDK installation shall complete within 60 seconds for versions under 500MB"

## Rationale

[Why this requirement exists. What problem does it solve? What value does it provide?]

## User Story (if applicable)

[For functional requirements]
As a [persona], I want [capability], so that [benefit].

[For non-functional requirements]
The system shall [constraint/quality attribute] to ensure [benefit/goal].

## Acceptance Criteria

[Specific, measurable, testable conditions that must be met]

- [ ] [Criterion 1 - be specific and testable]
- [ ] [Criterion 2 - include metrics where applicable]  
- [ ] [Criterion 3 - reference test names when known]
- [ ] [Criterion 4 - platform-specific behavior if needed]

## Technical Details (if applicable)

### Functional Requirement Details
[For FRs: Detailed behavior, inputs/outputs, error conditions]

### Non-Functional Requirement Details
[For NFRs: Specific constraints, thresholds, standards]
- Performance: [Latency/throughput targets]
- Security: [Security requirements, standards]
- Reliability: [Availability, retry behavior]
- Compatibility: [Platform-specific requirements]
- Usability: [UX requirements, message standards]

## Verification Method

### Test Strategy
- Test Type: Unit | Integration | Benchmark | Manual | E2E
- Test Location: `tests/[file].rs` or `src/[module].rs#[cfg(test)]`
- Test Names: `test_fr_<id>_description` or `bench_nfr_<id>_metric`

### Verification Commands
```bash
# Specific commands to verify this requirement
cargo test test_fr_<id>
cargo bench bench_nfr_<id>
# Platform-specific verification if needed
```

### Success Metrics
[How to measure that the requirement is successfully implemented]
- Metric 1: [Specific measurement and target]
- Metric 2: [Specific measurement and target]

## Dependencies

- Depends on: FR-<id>, NFR-<id> | N/A – No dependencies
- Blocks: FR-<id>, NFR-<id> | N/A – Blocks nothing

## Platform Considerations

### Unix
[Unix-specific behavior or requirements] | N/A – Platform agnostic

### Windows  
[Windows-specific behavior or requirements] | N/A – Platform agnostic

### Cross-Platform
[Behavior that must be consistent across platforms] | N/A – Platform agnostic

## Risks & Mitigation

| Risk | Impact | Likelihood | Mitigation | Validation |
|------|--------|------------|------------|------------|
| [Risk description] | High/Medium/Low | High/Medium/Low | [Mitigation strategy] | [How to verify mitigation] |

## Implementation Notes

[Any guidance for implementers. This is NOT a design document but can include:]
- Preferred approaches or patterns to follow
- Known pitfalls to avoid
- Related code areas or modules
- Suggested libraries or tools

## External References
<!-- Only external resources. Internal documents go in Links section -->
- [External specification or standard](URL) - Description | N/A – No external references

## Change History

[Tracked via Git. Major changes can be noted here for convenience]
- YYYY-MM-DD: Initial version
- YYYY-MM-DD: [Major change description]

---

## Template Usage

For detailed instructions, see [Template Usage Instructions](README.md#individual-requirement-template-requirementsmd) in the templates README.