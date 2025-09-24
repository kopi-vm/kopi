# Parallel Development Guide for TDL

This guide shows how to keep Traceable Development Lifecycle (TDL) artifacts consistent when multiple worktrees are active. It combines the random ID workflow with the expectations baked into `scripts/trace-status.ts` and the templates in `docs/templates/`.

## Why Parallel Worktrees Need Structure

Working in several git worktrees at the same time causes two recurring issues:

- Sequential IDs collide when separate branches add `AN-0001`, `FR-0002`, and so on.
- A shared `docs/traceability.md` file produces merge conflicts.

The Kopi TDL workflow solves both by using random document IDs, per-document traceability via Links sections, and on-demand status reports.

## Random ID Generation

Use the helper script whenever you create a new TDL document:

```bash
./scripts/tdl-new-id.ts
# Example output: a3bf2
```

Key points:

- IDs are 5-character base36 strings (\~60 million combinations).
- The script retries up to 10 times if it finds a collision.
- Optional overrides: set `DOCS_DIR` to scan an alternate tree or `ID_LEN` to change the length (invalid overrides fall back to 5).
- Keep existing sequential IDs; new work uses random IDs only.

## Always Start From Templates

Copy the matching template from `docs/templates/` before you edit anything:

```bash
cp docs/templates/requirements.md docs/requirements/FR-a3bf2-feature-name.md
cp docs/templates/plan.md docs/tasks/T-b4821-new-feature/plan.md
```

Replace every placeholder:

- Lines in backticks (`` `[text]` ``), bracketed values, and options that contain `|` are placeholders. Leaving them untouched causes `scripts/trace-status.ts` to treat metadata or status as unknown.
- When a section does not apply, delete it or write `N/A – <reason>` rather than leaving the template stub.

### Metadata Expectations

Every template begins with a Metadata block. `scripts/trace-status.ts` reads:

- `- Type:` — must contain the final value (for example `Functional Requirement`, `Design`, `Implementation Plan`).
- `- Status:` — pick the documented status options from the template you used. Common requirement statuses: `Proposed`, `Accepted`, `Implemented`, `Verified`, `Deprecated`.

Status values feed the "Status by Document Type" section in the trace-status output, so keep them current when work progresses.

### Links Section Format

Each template includes a `## Links` section. Keep the structure below so the script can classify relationships:

| Template label                          | Recognized as  |
| --------------------------------------- | -------------- |
| `Related Analyses`                      | `analyses`     |
| `Prerequisite Requirements`             | `depends_on`   |
| `Dependent Requirements`                | `blocks`       |
| `Related Requirements` / `Requirements` | `requirements` |
| `Related ADRs`                          | `adrs`         |
| `Related Tasks`                         | `tasks`        |

Guidelines:

- Start each entry with `- Label:` on one line, then list linked IDs as indented bullets (one ID per line).
- Use repository-relative markdown links when the artifact already exists. For future work, you can list the ID alone until the file is created.
- If there are no links for a label, remove that bullet or mark it `N/A – Not applicable`.

The script merges information from multiple files that share an ID. For tasks, it prioritizes `README.md`, then `plan.md`, then `design.md`. Keep Links and Status synchronized across those files to avoid confusing merges.

## Using `scripts/trace-status.ts`

The status script replaces the old static report and can be run from any subdirectory (it finds the repo root by locating `.git` or `Cargo.toml`). Typical usage:

```bash
./scripts/trace-status.ts            # Full status report
./scripts/trace-status.ts --gaps     # Only show missing links/gaps
./scripts/trace-status.ts --check    # CI mode; exits non-zero if issues exist
./scripts/trace-status.ts --check --write
./scripts/trace-status.ts --write=artifacts/trace.md
```

Behaviour summary:

- Loads analyses, requirements (FR/NFR), ADRs, and task documents (`README.md`, `plan.md`, `design.md`). Templates inside `docs/templates/` are ignored automatically.
- Prints coverage stats, gap summaries, dependency consistency warnings, and a per-type status histogram.
- Detects orphan requirements (no implementing tasks) and orphan tasks (no linked requirements).
- Validates reciprocal requirement dependencies. If `FR-abcde` lists `FR-fghij` as a prerequisite, the latter must list the former as a dependent. Missing links trigger warnings.
- `--check` is intended for CI: it writes findings to stderr and returns exit code `1` when gaps or inconsistent dependencies are present.
- `--write` generates a Markdown report (defaults to `docs/traceability.md` when no path is provided). This file stays in `.gitignore`; do not commit it.

The generated report includes:

1. Coverage summary
2. Requirement implementation table (Analyses ↔ ADRs ↔ Requirements ↔ Tasks with statuses)
3. Requirement dependency table and reciprocity guidance
4. Gap list and regeneration notice

## Parallel Workflow Checklist

Follow these steps in each worktree to avoid conflicts:

1. Generate a new ID with `./scripts/tdl-new-id.ts` before copying a template.
2. Fill out Metadata and Links immediately; placeholder text should never reach a merge request.
3. Keep Links reciprocal as you add dependencies. Update both requirement documents if you change prerequisites or dependents.
4. Run `./scripts/trace-status.ts --check` before pushing or opening a PR. Fix any reported gaps or dependency issues.
5. If you need a point-in-time report, run `./scripts/trace-status.ts --write` and share the generated file out-of-band without committing it.

## Migration Notes

- Existing sequential IDs remain valid; the status script recognises both `FR-0001` and `FR-a3bf2` formats.
- Only adopt the random ID workflow for brand-new documents—there is no need to rename historical files.
- If a legacy document lacks a `## Links` section or Metadata block, retrofit the template structure before making further edits so that automation continues to work.

By combining the random ID generator, the official templates, and the trace-status script, parallel worktrees stay independent while traceability remains complete and always up to date.
