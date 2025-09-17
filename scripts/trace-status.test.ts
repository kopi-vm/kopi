import { afterEach, beforeEach, describe, expect, it } from "bun:test";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, sep } from "node:path";

import {
  TraceabilityAnalyzer,
  capitalize,
  extractDocumentId,
  extractDocumentStatus,
  extractDocumentTitle,
  extractIds,
  findRepoRoot,
  inferDocumentType,
  main,
  parseArgs,
  parseDocumentLinks,
  resolveLinkType,
  resolveOutputPath,
  safeReadFile,
  toPosixPath,
  walkFiles,
} from "./trace-status";

const tempRoots: string[] = [];

function createTempDir(prefix = "trace-status-test-"): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
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

describe("extractDocumentId", () => {
  it("prefers identifier from filename", () => {
    const id = extractDocumentId(
      "FR-0001-feature.md",
      "/repo/docs/requirements/FR-0001-feature.md",
      null,
    );
    expect(id).toBe("FR-0001");
  });

  it("falls back to path segments", () => {
    const id = extractDocumentId(
      "plan.md",
      "/repo/docs/tasks/T-0a1b2-sample/plan.md",
      null,
    );
    expect(id).toBe("T-0a1b2");
  });

  it("reads metadata when no filename or path match", () => {
    const content = "# Document\n- ID: ADR-1234-some\n";
    const id = extractDocumentId("plan.md", "/repo/plan.md", content);
    expect(id).toBe("ADR-1234-some");
  });

  it("falls back to filename when no identifier found", () => {
    const id = extractDocumentId("notes.md", "/repo/notes.md", null);
    expect(id).toBe("notes.md");
  });
});

describe("inferDocumentType", () => {
  it("infers from filename prefix", () => {
    expect(inferDocumentType("AN-topic.md", "path")).toBe("analysis");
    expect(inferDocumentType("FR-123.md", "path")).toBe("requirement");
    expect(inferDocumentType("NFR-99.md", "path")).toBe("requirement");
    expect(inferDocumentType("ADR-12.md", "path")).toBe("adr");
    expect(inferDocumentType("T-abc.md", "path")).toBe("task");
  });

  it("infers tasks from directory structure", () => {
    const filePath = "docs/tasks/T-0001-sample/plan.md";
    expect(inferDocumentType("plan.md", filePath)).toBe("task");
  });

  it("returns unknown when inference fails", () => {
    expect(inferDocumentType("notes.md", "notes.md")).toBe("unknown");
  });
});

describe("parseDocumentLinks", () => {
  it("collects ids by link type", () => {
    const content =
      "# FR sample\n\n## Links\n- Requirements: FR-0001, FR-0002\n- Analysis: AN-0003\n- Tasks:\n  - T-0004\n  - T-0005\n- ADRs: ADR-0006\n";
    const links = parseDocumentLinks(content);
    expect(links).toEqual({
      requirements: ["FR-0001", "FR-0002"],
      analyses: ["AN-0003"],
      tasks: ["T-0004", "T-0005"],
      adrs: ["ADR-0006"],
    });
  });

  it("ignores entries when label is unknown", () => {
    const content = "## Links\n- Misc: FR-0001\n";
    const links = parseDocumentLinks(content);
    expect(links).toEqual({});
  });

  it("resets current section on blank lines", () => {
    const content = "## Links\n- Requirements: FR-0001\n\n- T: T-0002\n";
    const links = parseDocumentLinks(content);
    expect(links).toEqual({ requirements: ["FR-0001"] });
  });
});

describe("extractDocumentStatus", () => {
  it("returns status when present", () => {
    const status = extractDocumentStatus("- Status: In Progress  \n");
    expect(status).toBe("In Progress");
  });

  it("returns Unknown when absent", () => {
    expect(extractDocumentStatus("# Heading")).toBe("Unknown");
  });
});

describe("extractDocumentTitle", () => {
  it("extracts first heading", () => {
    const title = extractDocumentTitle("# Heading\nMore text");
    expect(title).toBe("Heading");
  });

  it("returns empty string when missing", () => {
    expect(extractDocumentTitle("No title")).toBe("");
  });
});

describe("safeReadFile", () => {
  it("returns file contents when readable", () => {
    const repo = createTempDir();
    const file = join(repo, "doc.md");
    writeFileSync(file, "hello", "utf8");
    expect(safeReadFile(file)).toBe("hello");
  });

  it("returns null and logs warning when unreadable", () => {
    const missing = join(createTempDir(), "missing.md");
    const errors: unknown[][] = [];
    const originalError = console.error;
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };
    try {
      expect(safeReadFile(missing)).toBeNull();
      expect(errors.length).toBeGreaterThan(0);
      expect(String(errors[0][0])).toContain("Warning: Failed to read");
    } finally {
      console.error = originalError;
    }
  });
});

describe("walkFiles", () => {
  it("yields only files and respects recursion flag", () => {
    const root = createTempDir();
    const base = join(root, "docs");
    writeDoc(root, "docs/file-a.txt", "a");
    writeDoc(root, "docs/sub/file-b.txt", "b");

    const nonRecursive = [...walkFiles(base, false)].map((p) =>
      relative(base, p),
    );
    expect(nonRecursive).toEqual(["file-a.txt"]);

    const recursive = [...walkFiles(base, true)].map((p) => relative(base, p));
    expect(recursive.sort()).toEqual(["file-a.txt", join("sub", "file-b.txt")]);
  });

  it("yields nothing for missing directories", () => {
    const root = createTempDir();
    const missing = join(root, "missing");
    expect([...walkFiles(missing, true)]).toEqual([]);
  });
});

describe("resolveLinkType", () => {
  it("normalizes labels", () => {
    expect(resolveLinkType("requirements")).toBe("requirements");
    expect(resolveLinkType("analysis details")).toBe("analyses");
    expect(resolveLinkType("adr references")).toBe("adrs");
    expect(resolveLinkType("task items")).toBe("tasks");
    expect(resolveLinkType("design outline")).toBe("design");
    expect(resolveLinkType("plan summary")).toBe("plan");
    expect(resolveLinkType("other")).toBeNull();
  });
});

describe("extractIds", () => {
  it("finds uppercase identifiers", () => {
    const value = "Links FR-0001 and ADR-abcde plus T-12345";
    expect(extractIds(value).sort()).toEqual([
      "ADR-abcde",
      "FR-0001",
      "T-12345",
    ]);
  });

  it("returns empty array when no matches", () => {
    expect(extractIds("none")).toEqual([]);
  });
});

describe("capitalize", () => {
  it("capitalizes first letter", () => {
    expect(capitalize("status")).toBe("Status");
  });

  it("returns input when empty", () => {
    expect(capitalize("")).toBe("");
  });
});

describe("findRepoRoot", () => {
  it("ascends directories until marker is found", () => {
    const repo = createTempDir();
    writeFileSync(join(repo, "Cargo.toml"), "", "utf8");
    const nested = join(repo, "nested", "deeper");
    mkdirSync(nested, { recursive: true });
    expect(findRepoRoot(nested)).toBe(repo);
  });

  it("returns null when no marker exists", () => {
    const dir = createTempDir();
    expect(findRepoRoot(dir)).toBeNull();
  });
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
    const repoRoot = createTempDir();

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

describe("main", () => {
  const originalArgv = process.argv.slice();
  const originalEnv = { ...process.env };
  const originalCwd = process.cwd();
  const originalLog = console.log;
  const originalError = console.error;
  let logCalls: unknown[][];
  let errorCalls: unknown[][];

  beforeEach(() => {
    logCalls = [];
    errorCalls = [];
    console.log = (...args: unknown[]) => {
      logCalls.push(args);
    };
    console.error = (...args: unknown[]) => {
      errorCalls.push(args);
    };
    process.argv = ["bun", "trace-status.ts"];
  });

  afterEach(() => {
    console.log = originalLog;
    console.error = originalError;
    process.argv = originalArgv.slice();
    process.chdir(originalCwd);

    for (const key of Object.keys(process.env)) {
      if (!(key in originalEnv)) {
        delete process.env[key];
      }
    }
    for (const [key, value] of Object.entries(originalEnv)) {
      process.env[key] = value;
    }
  });

  it("returns error when repository root is missing", () => {
    const cwd = createTempDir();
    process.chdir(cwd);

    const exitCode = main();

    expect(exitCode).toBe(1);
    expect(errorCalls.length).toBeGreaterThan(0);
    expect(String(errorCalls[0][0])).toContain(
      "Error: Could not find repository root",
    );
  });

  it("passes integrity check and writes report when requested", () => {
    const repo = createTempDir();
    writeFileSync(join(repo, "Cargo.toml"), "", "utf8");

    writeDoc(
      repo,
      "docs/requirements/FR-0001-ready.md",
      "# FR-0001 Ready\n- Status: Done\n\n## Links\n- Tasks: T-0001\n",
    );
    writeDoc(
      repo,
      "docs/tasks/T-0001-ready/plan.md",
      "# T-0001 Ready Plan\n- Status: Done\n\n## Links\n- Requirements: FR-0001\n",
    );

    const workingDir = join(repo, "subdir");
    mkdirSync(workingDir, { recursive: true });
    process.chdir(workingDir);

    process.argv.push("--check", "--write=reports/out.md");

    const exitCode = main();

    expect(exitCode).toBe(0);
    expect(errorCalls).toEqual([]);
    expect(
      logCalls.some((args) =>
        String(args[0]).includes("Traceability check passed; report written"),
      ),
    ).toBe(true);

    const reportPath = join(repo, "reports", "out.md");
    const report = readFileSync(reportPath, "utf8");
    expect(report).toContain("# Kopi Traceability Overview");
  });

  it("fails integrity check when gaps exist", () => {
    const repo = createTempDir();
    writeFileSync(join(repo, "Cargo.toml"), "", "utf8");
    writeDoc(
      repo,
      "docs/requirements/FR-0002-gap.md",
      "# FR-0002 Gap\n- Status: Draft\n",
    );

    process.chdir(repo);
    process.argv.push("--check");

    const exitCode = main();

    expect(exitCode).toBe(1);
    expect(errorCalls.length).toBeGreaterThan(0);
    expect(String(errorCalls[0][0])).toContain("Traceability gaps detected");
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

describe("toPosixPath", () => {
  it("converts platform-specific separators", () => {
    const systemPath = ["docs", "tasks", "T-0001", "plan.md"].join(sep);
    expect(toPosixPath(systemPath)).toBe("docs/tasks/T-0001/plan.md");
  });

  it("leaves POSIX paths unchanged", () => {
    const pathValue = "docs/tasks/T-0001/plan.md";
    expect(toPosixPath(pathValue)).toBe(pathValue);
  });
});
