# Parallel Development Guide for TDL

This guide explains how to handle parallel development using git-worktree with the Traceable Development Lifecycle (TDL).

## The Problem

When multiple developers or AI agents work in parallel using git-worktree:

1. **ID Collisions**: Sequential IDs (AN-0001, FR-0002) frequently collide
2. **Merge Conflicts**: Central `docs/traceability.md` causes constant conflicts

## The Solution

### 5-Character Random IDs

Instead of sequential numbers, use randomly generated 5-character IDs:

```bash
# Generate a unique ID for new documents
./scripts/tdl-new-id.ts
# Output example: a3bf2
```

> Tip: override `DOCS_DIR` to scan a different tree or `ID_LEN` to change the ID length
> (invalid overrides fall back to the five-character default).

**Characteristics:**

- **Format**: 5 random characters using base36 (0-9, a-z)
- **Namespace**: `~60 million` possible combinations
- **Collision probability**: `~1%` at 1,100 documents
- **Collision detection**: Script automatically checks for existing IDs

### Document Naming Convention

```text
AN-a3bf2-concurrent-locking.md     # Analysis
FR-b4cd8-user-authentication.md    # Functional Requirement
NFR-c5de9-performance.md            # Non-Functional Requirement
ADR-d6ef0-cache-strategy.md         # Architecture Decision Record
T-e7fa1-implement-locking/          # Task directory
```

### No Central Traceability File

Instead of maintaining a central `docs/traceability.md` that causes merge conflicts:

1. **Each document maintains its own Links section** (source of truth)
2. **Status viewed on-demand** using `scripts/trace-status.ts`
3. **File is in `.gitignore`** to prevent commits and conflicts

## Workflow

### Creating New Documents

1. **Generate a unique ID:**

   ```bash
   ./scripts/tdl-new-id.ts
   # Output: a3bf2
   ```

2. **Create document with the ID:**

   ```bash
   # Example for a new requirement
   cp docs/templates/requirements.md docs/requirements/FR-a3bf2-feature-name.md
   ```

3. **Fill in the Links section** to establish relationships

### Viewing Traceability Status

```bash
# View full traceability status
./scripts/trace-status.ts

# View only gaps (orphan requirements/tasks)
./scripts/trace-status.ts --gaps

# CI check mode (exits with error if gaps found)
./scripts/trace-status.ts --check

# Generate a report (defaults to docs/traceability.md without committing)
./scripts/trace-status.ts --check --write
```

### Example Output

```text
=== Kopi TDL Status ===

Coverage:
  Documents: 1 analyses, 8 requirements, 1 ADRs, 0 tasks
  Implementation: 0/8 requirements have tasks (0%)

Gaps:
  ⚠ FR-0001: No implementing task
  ⚠ FR-0002: No implementing task
  ...
```

## Benefits

✅ **No ID collisions** - Random IDs are unique across worktrees\
✅ **No merge conflicts** - No central file to conflict\
✅ **Parallel independence** - Each worktree operates independently\
✅ **Always current** - Status generated on-demand from source documents

## Migration from Sequential IDs

For existing documents with sequential IDs (AN-0001, FR-0001):

1. **Keep existing filenames** - No need to rename
2. **New documents use random IDs** - Start using the new system going forward
3. **Both formats work** - The trace-status.ts script handles both

## Implementation Details

### ID Generation Script

Location: `scripts/tdl-new-id.ts`

- TypeScript script executed via Bun (`#!/usr/bin/env bun`)
- Uses Bun's Web Crypto implementation to produce unbiased base36 output
- Scans `DOCS_DIR` (default: `docs`) once to avoid per-attempt filesystem walks
- Supports `ID_LEN` to override the default length (invalid values fall back with a warning)
- Retries up to 10 times if a collision is detected

### Status Display Script

Location: `scripts/trace-status.ts`

- Parses Links sections from all TDL documents (no frontmatter required)
- Calculates coverage statistics and highlights orphan requirements/tasks
- `--gaps` prints only the gap summary; `--check` exits non-zero when gaps remain
- `--write` and `--write=<path>` generate markdown reports without committing them

## FAQ

**Q: What if an ID collision occurs?**\
A: The script automatically detects and regenerates. With 5 characters, collision probability is negligible.

**Q: How do I see the full project status?**\
A: Run `./scripts/trace-status.ts` anytime for current status.

**Q: What about existing sequential IDs?**\
A: They continue to work. The system handles both formats.

**Q: Can I still generate a central traceability.md if needed?**\
A: Yes, the script can output to a file, but it should not be committed to avoid conflicts.
