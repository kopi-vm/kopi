import { afterEach, describe, expect, it } from "bun:test";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import {
  TraceabilityAnalyzer,
  parseArgs,
  resolveOutputPath,
} from "./trace-status";

const tempRoots: string[] = [];

function createTempRepo(): string {
  const dir = mkdtempSync(join(tmpdir(), "trace-status-test-"));
  tempRoots.push(dir);
  return dir;
}

function writeDoc(root: string, relativePath: string, content: string): void {
  const target = join(root, relativePath);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, content, "utf8");
}

afterEach(() => {
  while (tempRoots.length) {
    const dir = tempRoots.pop();
    if (!dir) {
      continue;
    }
    rmSync(dir, { recursive: true, force: true });
  }
});

describe("parseArgs", () => {
  it("parses supported flags", () => {
    const result = parseArgs(["--gaps", "--check", "--write=output.md"]);
    expect(result).toEqual({
      gapsOnly: true,
      checkMode: true,
      writePath: "output.md",
    });
  });

  it("supports bare --write", () => {
    const result = parseArgs(["--write"]);
    expect(result).toEqual({
      gapsOnly: false,
      checkMode: false,
      writePath: "",
    });
  });
});

describe("TraceabilityAnalyzer", () => {
  it("calculates coverage and detects gaps", () => {
    const repoRoot = createTempRepo();

    writeDoc(
      repoRoot,
      "docs/requirements/FR-0001-sample.md",
      "# FR-0001 Sample Requirement\n- ID: FR-0001-sample\n- Status: In Progress\n\n## Links\n- Tasks: T-0001\n- Analyses: AN-0001\n- ADRs:\n  - ADR-0001\n",
    );

    writeDoc(
      repoRoot,
      "docs/requirements/FR-0002-backlog.md",
      "# FR-0002 Backlog Item\n- Status: Backlog\n\n## Links\n- Analyses: AN-0001\n",
    );

    writeDoc(
      repoRoot,
      "docs/analysis/AN-0001-investigation.md",
      "# AN-0001 Investigation\n- Status: Complete\n\n## Links\n- Requirements: FR-0001\n",
    );

    writeDoc(
      repoRoot,
      "docs/adr/ADR-0001-decision.md",
      "# ADR-0001 Decision\n- Status: Accepted\n\n## Links\n- Requirements: FR-0001\n",
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-0001-demo/plan.md",
      "# T-0001 Demo Task\n- Status: Active\n\n## Links\n- Requirements: FR-0001\n",
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-0002-unlinked/plan.md",
      "# T-0002 Unlinked\n- Status: Draft\n",
    );

    const analyzer = new TraceabilityAnalyzer(repoRoot);
    const coverage = analyzer.calculateCoverage();
    expect(coverage.total_requirements).toBe(2);
    expect(coverage.requirements_with_tasks).toBe(1);
    expect(coverage.coverage_percentage).toBeCloseTo(50);
    expect(coverage.total_tasks).toBe(2);
    expect(coverage.total_analyses).toBe(1);
    expect(coverage.total_adrs).toBe(1);

    expect(analyzer.findOrphanRequirements()).toEqual(["FR-0002"]);
    expect(analyzer.findOrphanTasks()).toEqual(["T-0002"]);

    const outputPath = join(repoRoot, "docs", "traceability.md");
    const markdown = analyzer.renderTraceabilityMarkdown(outputPath);
    expect(markdown).toContain("| Requirements | 2 |");
    expect(markdown).toContain("| Requirements with tasks | 1 (50%) |");
    expect(markdown).toContain(
      "| [FR-0001](requirements/FR-0001-sample.md) - FR-0001 Sample Requirement | In Progress |",
    );
    expect(markdown).toContain("[T-0001](tasks/T-0001-demo/plan.md) (Active)");
    expect(markdown).toContain(
      "- FR-0002: No implementing task (Status: Backlog)",
    );
  });
});

describe("resolveOutputPath", () => {
  it("resolves repository-relative paths", () => {
    const repoRoot = "/tmp/repo";
    expect(resolveOutputPath("reports/trace.md", repoRoot)).toBe(
      join(repoRoot, "reports", "trace.md"),
    );
  });

  it("handles absolute paths", () => {
    const absolute = "/var/tmp/report.md";
    expect(resolveOutputPath(absolute, "/ignored")).toBe(absolute);
  });

  it("defaults bare write flag to docs/traceability.md", () => {
    const repoRoot = "/opt/kopi";
    expect(resolveOutputPath("", repoRoot)).toBe(
      join(repoRoot, "docs", "traceability.md"),
    );
  });
});
