# TDL Templates

This directory contains templates for the Traceable Development Lifecycle (TDL).

For complete TDL documentation and workflow, see [`../tdl.md`](../tdl.md).

## Available Templates

### Core Workflow Templates

- [`analysis.md`](analysis.md) - Template for exploratory analysis and problem space investigation
- [`requirements.md`](requirements.md) - Template for individual requirement documents (FR/NFR)
- [`design.md`](design.md) - Task-specific design document template
- [`plan.md`](plan.md) - Task-specific implementation plan template

### Architecture Decision Records

- [`adr.md`](adr.md) - Full ADR template for architecturally significant decisions
- [`adr-lite.md`](adr-lite.md) - Lightweight ADR for tactical choices with limited scope

## Template Usage Instructions

### Placeholder Conventions

Templates use the following placeholder conventions:

- **`` `[descriptive text]` ``** - Content placeholders that should be replaced with your actual content. The backticks make these visually distinct in the templates.
  - Example: `` `[Person or role]` `` → "John Smith" or "Engineering Lead"
  - Example: `` `[Performance target if applicable]` `` → "Response time < 200ms"

- **`<id>` or `<reason>`** - Short, single-word placeholders for IDs or brief values
  - Example: `FR-<id>` → `FR-001`
  - Example: `N/A – <reason>` → `N/A – Not yet implemented`

- **`[Link text]`** in markdown links - Standard markdown link syntax (not a placeholder)
  - Example: `[External resource title](https://example.com)` → `[AWS S3 Documentation](https://docs.aws.amazon.com/s3/)`

### Analysis Template (`analysis.md`)

1. Use for exploring problem spaces and discovering requirements
2. Include research, user feedback, technical investigations
3. Document discovered requirements as FR-DRAFT and NFR-DRAFT
4. Archive after requirements are formalized

### Individual Requirement Template (`requirements.md`)

1. One requirement per file for clear ownership and traceability
2. Define measurable acceptance criteria; keep brief and testable
3. Requirements are long-lived and can be referenced by multiple tasks over time
4. Task design/plan documents reference these requirement IDs rather than duplicating content
5. Prefer clarity and safety: English-only messaging, avoid "manager"/"util" naming, do not use `unsafe`

### ADR Templates (`adr.md` and `adr-lite.md`)

1. Use the Quick Selection Checklist below to choose between Full and Lite templates
2. One decision per ADR; evolve via `Status` and `Supersedes/Superseded by` links
3. Follow the Common Documentation Requirements for language, links, and traceability

**Required-if-Applicable Sections** (Full ADR only): The sections marked "(required if applicable)" - Platform Considerations, Security & Privacy, and Monitoring & Logging - must be filled out when relevant to your decision. If not applicable, you may remove these sections entirely.

#### Quick ADR Template Selection Checklist

**Use Full ADR if ANY of these apply:**

- [ ] Affects 3+ modules or components (quantitative threshold)
- [ ] Has security/privacy implications (risk level: Medium/High)
- [ ] Requires platform-specific handling (Unix/Windows differences)
- [ ] Has 3+ viable alternatives with significant trade-offs
- [ ] Establishes patterns used across the codebase
- [ ] Changes public API or CLI interface
- [ ] Impacts error handling or exit codes
- [ ] Requires monitoring/logging considerations
- [ ] Reversibility effort > 8 hours of work

**Use Lite ADR if ALL of these apply:**

- [ ] Affects single module/component
- [ ] Clear best practice exists
- [ ] Low risk (easily reversible, < 8 hours to revert)
- [ ] No significant trade-offs (< 3 alternatives)
- [ ] No platform-specific considerations
- [ ] Internal implementation detail only

#### Detailed ADR Selection Criteria

- Use the Full ADR when decisions are:
  - Architecturally significant
  - Broad in impact across modules/platforms
  - Involve important trade‑offs or multiple viable options
  - Establish long‑lived patterns or policies
- Use the Lite ADR when decisions are:
  - Tactical and localized in scope
  - Low risk and aligned with established conventions
  - Straightforward with a clear best practice

### Design Template (`design.md`)

1. Reference requirement IDs (FR-<id>/NFR-<id>) in the Requirements Summary section
2. Link to relevant ADRs and create new ones when this design introduces material decisions
3. Capture concrete acceptance/success metrics to enable verification
4. Call out platform differences explicitly when touching shell, shims, filesystem, or paths
5. Specify testing strategy early, including external API parsing tests if applicable
6. Prefer clarity and safety over micro-optimizations; avoid `unsafe`, avoid vague names like "manager"/"util", and prefer functions for stateless behavior

### Plan Template (`plan.md`)

1. Reference requirement IDs (FR-<id>/NFR-<id>) being implemented
2. Adjust the number of phases based on complexity
3. Break down work into specific, testable items
4. Define verification commands and phase acceptance criteria
5. Identify risks early, with mitigation and fallback
6. Keep status updated as work progresses
7. Phase independence: Ensure each phase is self-contained; the `/clear` command may be executed at phase boundaries to reset context
8. Update or add ADRs when design decisions change
9. Error Recovery Patterns:
   - When blocked during implementation:
     a. Document blocker in current phase status
     b. Create new analysis document for the blocker if needed
     c. Generate new requirements if applicable (e.g., NFR for error handling)
     d. Update plan with mitigation steps

## Common Documentation Requirements

These requirements apply to ALL documentation templates:

### Document Structure

- **Metadata**: Include Type/Owner/Reviewers/Status consistently at the top
- **Document IDs**: Must be in the filename and Metadata section (not in document titles)
- **Links Section**: Mandatory in every template for traceability. If something doesn't apply, write: `N/A – <reason>`
- **Change History**: Use Git history (`git log --follow <file>`)

### Writing Standards

- **Language**: All documentation must be written in English (per `CLAUDE.md` policy)
- **Date Format**: Use `YYYY-MM-DD` format consistently
- **IDs & Naming**: Use explicit, stable IDs/names. Avoid vague terms like "manager" or "util"
- **Consistency**: Don't duplicate requirements text; Design references requirement IDs; Plan references both

### Markdown Formatting Guidelines

Use inline code (`` ` ``) for the following cases to ensure proper formatting and readability:

- **Environment Variables**: Always use inline code for environment variable names, especially those containing underscores
  - Example: `RUST_LOG`, `KOPI_HOME`, `RUST_TEST_THREADS`

- **Code Identifiers**: Use inline code for all programming language identifiers
  - Rust structs, traits, functions: `KopiError`, `ErrorContext`, `find_symbol()`
  - Command names and flags: `cargo test`, `--verbose`, `-D warnings`
  - File paths and extensions: `src/main.rs`, `.toml`, `~/.kopi/`

- **Special Characters**: Use inline code when describing text containing special characters
  - Version strings with special chars: `temurin@21`, `~/.kopi/jdks/`
  - Comparison operators: `< 200ms`, `> 8 hours`
  - Shell operators and paths: `&&`, `|`, `./scripts/`

- **Command Output**: Use inline code for inline examples of standard output or error messages
  - Example: The command returns `0` on success or `exit code 2` for invalid input
  - For multi-line output, use code blocks instead

- **Technical Terms with Symbols**: Use inline code for technical terms containing symbols
  - Package versions: `v1.2.3`, `^2.0.0`
  - Git references: `HEAD`, `main`, `ADR-<id>`

### Linking & Cross-References

- **Cross-linking**: Use relative links between documents
- **Links vs External References**: Maintain clear distinction:
  - **Links**: Internal project artifacts only (files in repo, issues, PRs)
  - **External References**: External resources only (standards, articles, documentation)
- **ID Usage**: Use FR/NFR/ADR IDs throughout documentation
- **Document Flow**: Cross-link Requirements → Design → Plan documents
- **Requirements Mapping**: Include Requirements Mapping table in Design documents
- **Test References**: Reference IDs in tests where feasible

### Process Requirements

- **Verification**: Use canonical cargo commands from `CLAUDE.md` in Verification blocks and Definition of Done
- **PR Integration**: Link Requirements/Design/Plan and relevant ADRs in PRs

## Examples

### Template Examples

#### Core Workflow Template Examples

- Analysis: [`examples/analysis-example.md`](examples/analysis-example.md) - Problem exploration and requirement discovery
- Individual Requirement: [`examples/requirement-example.md`](examples/requirement-example.md) - Single requirement document (e.g., FR-0001-user-authentication, NFR-0001-performance)
- Design: [`examples/design-example.md`](examples/design-example.md) - Task-specific technical design referencing requirement IDs
- Plan: [`examples/plan-example.md`](examples/plan-example.md) - Task-specific phased implementation with verification steps

#### ADR Templates

- Full ADR: [`examples/adr-full-example.md`](examples/adr-full-example.md) - Demonstrates all sections
- Lite ADR: [`examples/adr-lite-example.md`](examples/adr-lite-example.md) - Lightweight format for simple decisions

### Real Project Examples (Archived)

- Error Handling: [`../archive/adr/004-error-handling-strategy.md`](../archive/adr/004-error-handling-strategy.md) - Full ADR with multiple options analyzed
- Logging Strategy: [`../archive/adr/009-logging-strategy.md`](../archive/adr/009-logging-strategy.md) - Comprehensive platform considerations
- Configuration: [`../archive/adr/014-configuration-and-version-file-formats.md`](../archive/adr/014-configuration-and-version-file-formats.md) - Focused scope with clear trade-offs
