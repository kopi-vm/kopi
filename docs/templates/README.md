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

`scripts/trace-status.ts` treats any untouched placeholder (text wrapped in backticks, angle brackets, or pipes) as missing metadata. Always replace, delete, or mark placeholders `N/A – <reason>` before you commit.

### Generating Document IDs

- Use `./scripts/tdl-new-id.ts` to create 5-character base36 IDs before copying a template.
- Keep historical sequential IDs intact; the trace tooling accepts both formats.
- Embed the ID in the filename exactly as generated (for example `FR-a3bf2-feature-name.md`).

### Metadata Requirements

Every template starts with a Metadata block that feeds the traceability tooling:

- `- Type:` must contain the final document type (for example `Functional Requirement`, `Design`, `Implementation Plan`).
- `- Status:` must match one of the allowed values in the template comments. Current status sets:
  - `analysis.md`: `Draft`, `Complete`, `Cancelled`, `Archived`
  - `requirements.md`: `Draft`, `Approved`, `Rejected`
  - `design.md`: `Draft`, `Approved`, `Rejected`
  - `plan.md`: `Draft`, `Phase X In Progress`, `Cancelled`, `Complete`
  - `task.md`: `Draft`, `In Progress`, `Complete`, `Cancelled`
  - `adr.md` / `adr-lite.md`: `Draft`, `Approved`, `Rejected`, `Deprecated`, `Superseded`
- Remove unused metadata fields or mark them explicitly as `N/A – <reason>`.

Leaving placeholders in Metadata causes `scripts/trace-status.ts` to report `Unknown` status or type for the document.

### Links Section Expectations

Every template ships with a `## Links` section. Use the exact labels found in the template files; the trace tooling depends on them for classification.

- `analysis.md`: `Related Analyses`, `Related Requirements`, `Related ADRs`, `Related Tasks`
- `requirements.md`: `Prerequisite Requirements`, `Dependent Requirements`, `Related Tasks`
- `adr.md` / `adr-lite.md`: `Impacted Requirements`, `Supersedes ADRs`, `Related Tasks`
- `design.md`: `Associated Plan Document`
- `plan.md`: `Associated Design Document`
- `task.md`: `Associated Plan Document`, `Associated Design Document`

Guidelines:

- Keep each label as a top-level bullet (`- Label:`) with IDs or file links indented beneath it.
- Use repository-relative links for existing artifacts; for items that do not exist yet, list the ID alone.
- If a label does not apply, replace the placeholder with `N/A – <reason>` rather than deleting the label.
- Tasks often include multiple documents (`README.md`, `plan.md`, `design.md`); ensure Links and Status stay in sync across them. When the trace script merges task files it prioritizes `README.md`, then `plan.md`, then `design.md`.

### Agent Usage (Claude & Codex)

Both AI collaborators follow the same TDL conventions. Use this checklist whenever Claude or Codex generates or edits documentation:

- Start by running `./scripts/tdl-new-id.ts` to obtain a fresh ID before copying any template.
- Copy the appropriate template, replace every placeholder, and update Metadata and Links immediately.
- When documentation changes accompany code, queue the required verification commands from this README (`bun format`, `bun lint`) plus language-specific toolchains noted in `CLAUDE.md` (Claude) or `AGENTS.md` (Codex).
- Before handing off between agents, run `./scripts/trace-status.ts --check` to confirm there are no orphan links or placeholder leftovers.
- Note any template sections intentionally removed or marked `N/A – <reason>` so the next agent retains context without reopening the template.

### Analysis Template (`analysis.md`)

1. Use for exploring problem spaces and discovering requirements
2. Include research, user feedback, technical investigations
3. Document discovered requirements as FR-DRAFT and NFR-DRAFT
4. Archive after requirements are formalized
5. Link active or resulting work items under `Related Tasks` to keep traceability intact

### Individual Requirement Template (`requirements.md`)

1. One requirement per file for clear ownership and traceability
2. Define measurable acceptance criteria; keep brief and testable
3. Requirements are long-lived and can be referenced by multiple tasks over time
4. Task documents reference these requirement IDs rather than duplicating content
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

### Task Template (`task.md`)

1. Create as soon as a requirement or ADR produces actionable work; this is the mandatory bridge into implementation.
2. Populate Metadata, Links, Summary, Scope, and Success Metrics before opening downstream work items.
3. Capture upstream references in the Links section (`Related Analyses`, `Related Requirements`, `Related ADRs`) and keep `Associated Plan Document` / `Associated Design Document` current.
4. Use the Detailed Plan section for preliminary milestone notes, but migrate decisions into `design.md`, then `plan.md` within the same task directory before coding starts.
5. Keep Notes for risks, follow-up decisions, coordination details, and blockers discovered later in the workflow.

### Design Template (`design.md`)

1. Author only after the task README exists; reference the same `T-<id>-<name>` directory.
2. Reference requirement IDs (FR-<id>/NFR-<id>) in the Requirements Summary and link to the task README and relevant ADRs.
3. Capture measurable success metrics to guide verification.
4. Call out platform differences explicitly when touching shell, shims, filesystem, or paths.
5. Specify testing strategy early, including external API parsing tests when applicable.
6. Prefer clarity and safety over micro-optimizations; avoid `unsafe`, avoid vague names like "manager"/"util", and prefer functions for stateless behavior.
7. Mark the design as complete and circulate for review before drafting `plan.md`; implementation work may only start after both documents exist.

### Plan Template (`plan.md`)

1. Draft only after the corresponding `design.md` is approved; link back to the task README and design in Metadata and Links.
2. Reference requirement IDs (FR-<id>/NFR-<id>) being implemented and align subtasks with the design decisions.
3. Adjust the number of phases based on complexity; each phase must be independently verifiable.
4. Break down work into observable, testable items with explicit verification commands.
5. Identify risks early, with mitigation, validation, and fallback noted inline.
6. Keep status updated as work progresses; annotate deferred items instead of leaving checkboxes blank.
7. Update or add ADRs if implementation deviates from approved design; reflect changes back in the task README and design.
8. Follow documented error recovery patterns:
   - Document blockers in the current phase status.
   - Create analysis docs for new investigations.
   - Publish new requirements if new constraints emerge.
   - Update plan milestones with mitigation steps.

## Common Documentation Requirements

These requirements apply to ALL documentation templates:

### Document Structure

- **Metadata**: Include Type/Owner/Reviewers/Status consistently at the top
- **Document IDs**: Must be embedded in the filename; referencing them elsewhere is optional unless a template explicitly calls it out
- **Links Section**: Mandatory in every template for traceability. If something doesn't apply, write: `N/A – <reason>`
- **Change History**: Use Git history (`git log --follow <file>`)

### Writing Standards

- **Language**: All documentation must be written in English (per `CLAUDE.md` and `AGENTS.md`)
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

- **Verification**: Use canonical cargo commands referenced in `CLAUDE.md` (Claude) or `AGENTS.md` (Codex) in Verification blocks and Definition of Done
- **PR Integration**: Link Requirements/Design/Plan and relevant ADRs in PRs

## Examples

Template-specific example files were removed to reduce maintenance noise. Refer to active or archived project documents for canonical usage patterns.

### Real Project Examples (Archived)

- Error Handling: [`../archive/adr/004-error-handling-strategy.md`](../archive/adr/004-error-handling-strategy.md) - Full ADR with multiple options analyzed
- Logging Strategy: [`../archive/adr/009-logging-strategy.md`](../archive/adr/009-logging-strategy.md) - Comprehensive platform considerations
- Configuration: [`../archive/adr/014-configuration-and-version-file-formats.md`](../archive/adr/014-configuration-and-version-file-formats.md) - Focused scope with clear trade-offs
