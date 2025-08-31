# AGENTS.md

## Template-Driven Workflow (Requirements → Design → Plan)

Use the templates in `docs/templates/` to keep scope, architecture, and execution aligned. All documents must be in English (see `CLAUDE.md` for the documentation language policy and repo-wide guidance).

Note: For repository-specific conventions (commands, workflow, architecture, error handling, naming), consult `CLAUDE.md` as the authoritative reference.

### Document Locations
- Requirements: `docs/tasks/<task>/requirements.md` (copy from `docs/templates/requirements.md`)
- Design: `docs/tasks/<task>/design.md` (copy from `docs/templates/design.md`)
- Plan: `docs/tasks/<task>/plan.md` (copy from `docs/templates/plan.md`)

### Step 1: Requirements (what/why)
- Define problem, scope (goals/non-goals), assumptions, and constraints.
- Create Functional Requirements (FR-###) and Non-Functional Requirements (NFR-###) with measurable acceptance criteria.
- Specify CLI/UX, Data/API, and Platform Matrix (Unix/Windows/filesystem) requirements as needed.
- Provide a verification plan (how acceptance will be checked) using cargo commands and tests. See `CLAUDE.md` (Development Commands/Workflow) for the canonical command set and ordering.

### Step 2: Design (how/trade-offs)
- Reference requirement IDs instead of duplicating text (see “Requirements Summary”).
- Document architecture, key components, data flows, storage paths, CLI/API argument shapes, models, and error handling (KopiError + ErrorContext + exit codes). Reference `CLAUDE.md` for existing architecture, storage locations, and error handling guidelines to avoid duplication.
- Record alternatives and trade-offs; link or add ADRs for material decisions.
- Add a Requirements Mapping table (FR/NFR → design section → tests/benches).

### Step 3: Plan & Execution (phases/tasks)
- Break work into phases with inputs, tasks, deliverables, Verification blocks, and Acceptance Criteria (phase gates).
- Use Verification commands consistently (see also `CLAUDE.md` – Development Commands/Workflow):
  - `cargo check`, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`
  - `cargo test --lib --quiet`, `cargo it`, `cargo perf`, `cargo bench` (as applicable)
- Include a Definition of Done at the end of the plan (final shipment criteria): tests green, docs updated (`docs/reference.md` and user docs if needed), ADRs updated, platform verification, no `unsafe`, no vague naming (no “manager”/“util”).

### Traceability & Reviews
- Requirements → Design → Plan must reference each other (links at the top of each file).
- Tests should reference FR/NFR IDs in names or comments when feasible.
- Reviews occur in order: Requirements (scope), then Design (architecture), then Plan (execution). 

### Pull Request Checklist
- Link the task’s requirements, design, and plan documents in the PR description.
- Verify DoD in the plan is satisfied:
  - `cargo check`, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`
  - Unit/integration/perf/bench tests as applicable (`cargo test --lib --quiet`, `cargo it`, `cargo perf`, `cargo bench`)
  - Error messages clear and in English (per `CLAUDE.md` language policy); exit codes correct via ErrorContext
  - Documentation updated (`docs/reference.md`, user docs repo if user-facing)
  - ADRs added/updated for design decisions
  - Platform behavior validated (Unix/Windows/filesystem when relevant)

### Small Changes Variant
- For trivial fixes (e.g., typo, log message, small refactor), you may skip full requirements/design.
- Still update the plan with a minimal Phase and DoD, and ensure all verification commands pass.
