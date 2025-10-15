# Parallel Development Guide for TDL

This guide complements `docs/tdl.md` for contributors who keep multiple git worktrees open. It highlights the minimum practices that prevent traceability conflicts while you work in parallel.

## Core Practices

- **Generate unique IDs first** – Run `./scripts/tdl-new-id.ts` before copying any template. The script and its options are documented in `docs/tdl.md`.
- **Start from the official templates** – Copy the template under `docs/templates/`, replace every placeholder, and update Metadata and Links immediately. Follow the template guidance in `docs/templates/README.md`.
- **Document ad hoc tasks on creation** – When urgent work starts at the task layer without upstream analysis/requirements/ADRs, create the task README right away and keep it linked. The default `bun scripts/trace-status.ts` output will keep flagging the missing upstream relationship until you either add the source document or finish the task.

## Traceability Checks

Run the status script regularly to avoid surprises:

```bash
bun scripts/trace-status.ts            # Gaps + consistency highlights
bun scripts/trace-status.ts --status   # Include coverage and per-type counts
bun scripts/trace-status.ts --check    # CI/hand-off gate; exits non-zero on gaps
bun scripts/trace-status.ts --write    # Optional point-in-time report (defaults to docs/traceability.md)
```

Use the script before handing work off, before opening a PR, and whenever you finish a task in a secondary worktree.

## Worktree Routine

1. Generate an ID, copy the template, and fill Metadata + Links immediately.
2. Keep reciprocal Links up to date as dependencies change.
3. Run `bun scripts/trace-status.ts --check` prior to pushing; resolve every reported gap.
4. If you need to share a report, run `--write` and distribute the generated file out of band (the output stays in `.gitignore`).

## Legacy Documents

- Historical sequential IDs remain valid; only new documents need random IDs.
- Retrofit Metadata and Links to the current templates before editing older files so automation keeps working.
- Remove duplicate local copies of `docs/traceability.md`; regenerate on demand with `bun scripts/trace-status.ts --write` when needed.
