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
  capitalize,
  calculateCoverage,
  collectTaskDesignPlanIssues,
  collectTaskReciprocalLinkIssues,
  extractDocumentId,
  extractDocumentStatus,
  extractDocumentTitle,
  extractFirstLineId,
  extractIds,
  findRepoRoot,
  findImplementingTasks,
  findOrphanAdrs,
  findOrphanRequirements,
  findOrphanTasks,
  findHeadingMismatches,
  inferDocumentType,
  loadDocuments,
  main,
  parseArgs,
  parseDocumentLinks,
  checkIntegrity,
  resolveLinkType,
  resolveOutputPath,
  renderTraceabilityMarkdown,
  printStatus,
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

type LinkValue = string | string[];

function renderLink(label: string, value: LinkValue): string[] {
  if (Array.isArray(value)) {
    if (value.length === 0) {
      return [`- ${label}: N/A – None`];
    }
    return [`- ${label}:`, ...value.map((entry) => `  - ${entry}`)];
  }
  return [`- ${label}: ${value}`];
}

function requirementDoc({
  id,
  title,
  status,
  type = "Functional Requirement",
  prerequisites = "N/A – None",
  dependents = "N/A – None",
  tasks = "N/A – Not yet planned",
  statement = "This requirement describes the expected behaviour for this capability.",
  rationale = "Captures the business value and technical motivation for the requirement.",
  acceptanceCriteria = [
    "Happy path scenario is implemented.",
    "Edge cases are covered with automated tests.",
  ],
}: {
  id: string;
  title: string;
  status: string;
  type?: string;
  prerequisites?: LinkValue;
  dependents?: LinkValue;
  tasks?: LinkValue;
  statement?: string;
  rationale?: string;
  acceptanceCriteria?: string[];
}): string {
  const lines: string[] = [
    `# ${id} ${title}`,
    "",
    "## Metadata",
    "",
    `- Type: ${type}`,
    `- Status: ${status}`,
    "",
    "## Links",
    "",
  ];
  lines.push(...renderLink("Prerequisite Requirements", prerequisites));
  lines.push(...renderLink("Dependent Requirements", dependents));
  lines.push(...renderLink("Related Tasks", tasks));
  lines.push("");
  lines.push("## Requirement Statement");
  lines.push("");
  lines.push(statement);
  lines.push("");
  lines.push("## Rationale");
  lines.push("");
  lines.push(rationale);
  lines.push("");
  lines.push("## Acceptance Criteria");
  lines.push("");
  for (const criterion of acceptanceCriteria) {
    lines.push(`- [ ] ${criterion}`);
  }
  lines.push("");
  lines.push("## Implementation Notes");
  lines.push("");
  lines.push("Document implementation guidance as the requirement progresses.");
  lines.push("");
  lines.push("## External References");
  lines.push("");
  lines.push("N/A – No external references.");
  return lines.join("\n");
}

function analysisDoc({
  id,
  title,
  status,
  relatedRequirements = "N/A – None yet identified",
  relatedAdrs = "N/A – None yet recorded",
  relatedAnalyses = "N/A – No prior analyses",
}: {
  id: string;
  title: string;
  status: string;
  relatedRequirements?: LinkValue;
  relatedAdrs?: LinkValue;
  relatedAnalyses?: LinkValue;
}): string {
  const lines: string[] = [
    `# ${id} ${title}`,
    "",
    "## Metadata",
    "",
    "- Type: Analysis",
    `- Status: ${status}`,
    "",
    "## Links",
    "",
  ];
  lines.push(...renderLink("Related Analyses", relatedAnalyses));
  lines.push(...renderLink("Related Requirements", relatedRequirements));
  lines.push(...renderLink("Related ADRs", relatedAdrs));
  lines.push("");
  lines.push("## Executive Summary");
  lines.push("");
  lines.push(
    "Summarises the investigation outcomes and recommended follow-up actions.",
  );
  lines.push("");
  lines.push("## Problem Space");
  lines.push("");
  lines.push("### Current State");
  lines.push("");
  lines.push(
    "Current behaviour is documented to provide context for the analysis.",
  );
  lines.push("");
  lines.push("### Desired State");
  lines.push("");
  lines.push(
    "Desired improvements are described for comparison against the current state.",
  );
  lines.push("");
  lines.push("### Gap Analysis");
  lines.push("");
  lines.push(
    "Highlights the differences between current and desired outcomes.",
  );
  lines.push("");
  lines.push("## Stakeholder Analysis");
  lines.push("");
  lines.push("| Stakeholder | Interest/Need | Impact | Priority |");
  lines.push("| --- | --- | --- | --- |");
  lines.push(
    "| Engineering | Clear direction for implementation | High | P0 |",
  );
  lines.push("");
  lines.push("## Recommendations");
  lines.push("");
  lines.push("1. Formalise requirements based on validated findings.");
  lines.push("2. Capture architectural implications through ADRs if needed.");
  return lines.join("\n");
}

function adrDoc({
  id,
  title,
  status,
  impactedRequirements = "N/A – Constraint only",
  supersedes = "N/A – None",
  relatedTasks = "N/A – No tasks linked yet",
}: {
  id: string;
  title: string;
  status: string;
  impactedRequirements?: LinkValue;
  supersedes?: LinkValue;
  relatedTasks?: LinkValue;
}): string {
  const lines: string[] = [
    `# ${id} ${title}`,
    "",
    "## Metadata",
    "",
    "- Type: ADR",
    `- Status: ${status}`,
    "",
    "## Links",
    "",
  ];
  lines.push(...renderLink("Impacted Requirements", impactedRequirements));
  lines.push(...renderLink("Supersedes ADRs", supersedes));
  lines.push(...renderLink("Related Tasks", relatedTasks));
  lines.push("");
  lines.push("## Context");
  lines.push("");
  lines.push("Explains the architectural forces that motivate this decision.");
  lines.push("");
  lines.push("## Decision");
  lines.push("");
  lines.push("States the chosen direction in clear, actionable language.");
  lines.push("");
  lines.push("## Rationale");
  lines.push("");
  lines.push("Describes why this option was selected over alternatives.");
  lines.push("");
  lines.push("## Consequences");
  lines.push("");
  lines.push("### Positive");
  lines.push("");
  lines.push("- Supports future maintenance.");
  lines.push("");
  lines.push("### Negative");
  lines.push("");
  lines.push("- Introduces migration work for existing components.");
  lines.push("");
  lines.push("## Open Questions");
  lines.push("");
  lines.push("- [ ] Track follow-up items as the implementation evolves.");
  lines.push("");
  lines.push("## External References");
  lines.push("");
  lines.push("N/A – No external references.");
  return lines.join("\n");
}

function taskReadmeDoc({
  id,
  title,
  status,
  planPath = `../tasks/${id.toLowerCase()}-${title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}/plan.md`,
  designPath = `../tasks/${id.toLowerCase()}-${title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}/design.md`,
}: {
  id: string;
  title: string;
  status: string;
  planPath?: string;
  designPath?: string;
}): string {
  const lines: string[] = [
    `# ${title}`,
    "",
    "## Metadata",
    "",
    "- Type: Task",
    `- Status: ${status}`,
    "",
    "## Links",
    "",
  ];
  lines.push(
    ...renderLink("Associated Plan Document", [`[${id}-plan](${planPath})`]),
  );
  lines.push(
    ...renderLink("Associated Design Document", [
      `[${id}-design](${designPath})`,
    ]),
  );
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push("Outlines the objective and desired outcome for the task.");
  lines.push("");
  lines.push("## Scope");
  lines.push("");
  lines.push("- In scope: Define and implement the required changes.");
  lines.push("- Out of scope: Unrelated refactors.");
  lines.push("");
  lines.push("## Success Metrics");
  lines.push("");
  lines.push("- Completion criteria are met.");
  lines.push("- Verification steps pass without regressions.");
  return lines.join("\n");
}

function taskPlanDoc({
  id,
  title,
  status,
  associatedDesign = "N/A – Awaiting design approval",
  requirements = "N/A – Requirements pending",
}: {
  id: string;
  title: string;
  status: string;
  associatedDesign?: LinkValue;
  requirements?: LinkValue;
}): string {
  const lines: string[] = [
    `# ${id} ${title}`,
    "",
    "## Metadata",
    "",
    "- Type: Implementation Plan",
    `- Status: ${status}`,
    "",
    "## Links",
    "",
  ];
  lines.push(...renderLink("Associated Design Document", associatedDesign));
  lines.push(...renderLink("Related Requirements", requirements));
  lines.push("");
  lines.push("## Overview");
  lines.push("");
  lines.push("Summarises the planned implementation approach for this task.");
  lines.push("");
  lines.push("## Success Metrics");
  lines.push("");
  lines.push("- [ ] Implementation delivers the targeted capability.");
  lines.push("- [ ] Tests confirm stability across supported platforms.");
  lines.push("");
  lines.push("## Scope");
  lines.push("");
  lines.push("- Goal: Complete the implementation steps.");
  lines.push("- Non-Goals: Any unrelated cleanups.");
  lines.push("- Assumptions: Required prerequisites are in place.");
  lines.push("- Constraints: Follow Kopi coding standards.");
  lines.push("");
  lines.push("## Plan Summary");
  lines.push("");
  lines.push("- Phase 1 – Preparation");
  lines.push("- Phase 2 – Implementation");
  lines.push("- Phase 3 – Verification");
  lines.push("");
  lines.push("## Phase 1: Preparation");
  lines.push("");
  lines.push("### Tasks");
  lines.push("");
  lines.push("- [ ] Finalise requirements alignment.");
  lines.push("- [ ] Confirm environment readiness.");
  lines.push("");
  lines.push("## Phase 2: Implementation");
  lines.push("");
  lines.push("### Tasks");
  lines.push("");
  lines.push("- [ ] Implement functionality.");
  lines.push("- [ ] Update documentation.");
  lines.push("");
  lines.push("## Phase 3: Testing & Integration");
  lines.push("");
  lines.push("### Tasks");
  lines.push("");
  lines.push("- [ ] Execute automated tests.");
  lines.push("- [ ] Validate cross-platform behaviour.");
  lines.push("");
  lines.push("## Testing Strategy");
  lines.push("");
  lines.push("- Unit tests cover new logic.");
  lines.push("- Integration tests confirm end-to-end flows.");
  lines.push("");
  lines.push("## Risk Assessment");
  lines.push("");
  lines.push("| Risk | Mitigation | Validation |");
  lines.push("| --- | --- | --- |");
  lines.push("| Missed edge cases | Peer review | Automated tests |");
  lines.push("");
  lines.push("## Dependencies");
  lines.push("");
  lines.push("- Requirements:");
  if (Array.isArray(requirements)) {
    for (const req of requirements) {
      lines.push(`  - ${req}`);
    }
  } else {
    lines.push(`  - ${requirements}`);
  }
  return lines.join("\n");
}

function taskDesignDoc({
  id,
  title,
  status,
  associatedPlan = "N/A – Plan pending",
}: {
  id: string;
  title: string;
  status: string;
  associatedPlan?: LinkValue;
}): string {
  const lines: string[] = [
    `# ${id} ${title}`,
    "",
    "## Metadata",
    "",
    "- Type: Design",
    `- Status: ${status}`,
    "",
    "## Links",
    "",
  ];
  lines.push(...renderLink("Associated Plan Document", associatedPlan));
  lines.push("\n## Overview\n");
  lines.push(
    "Describes the architectural approach and key design decisions for the task.",
  );
  lines.push("\n## Components\n");
  lines.push("- Document the primary modules and their responsibilities.");
  lines.push("\n## Risks\n");
  lines.push("- [ ] Capture open questions and validation steps.");
  return lines.join("\n");
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
  it("collects task links from a requirement document", () => {
    const content = requirementDoc({
      id: "FR-0001",
      title: "Sample Requirement",
      status: "Accepted",
      tasks: [
        "[T-0004-demo](../tasks/T-0004-demo/plan.md)",
        "[T-0005-followup](../tasks/T-0005-followup/plan.md)",
      ],
    });
    const links = parseDocumentLinks(content);
    const uniqueTasks = [...new Set(links.tasks ?? [])];
    expect(uniqueTasks).toEqual(["T-0004", "T-0005"]);
  });

  it("collects requirement and ADR links from an analysis document", () => {
    const content = analysisDoc({
      id: "AN-0003",
      title: "Investigation",
      status: "Complete",
      relatedRequirements: [
        "[FR-0001](../requirements/FR-0001-sample.md)",
        "[FR-0002](../requirements/FR-0002-backlog.md)",
      ],
      relatedAdrs: ["[ADR-0006](../adr/ADR-0006-decision.md)"],
      relatedAnalyses: "N/A – No previous analysis",
    });
    const links = parseDocumentLinks(content);
    const uniqueRequirements = [...new Set(links.requirements ?? [])];
    const uniqueAdrs = [...new Set(links.adrs ?? [])];
    expect(uniqueRequirements).toEqual(["FR-0001", "FR-0002"]);
    expect(uniqueAdrs).toEqual(["ADR-0006"]);
    expect(links.analyses ?? []).toEqual([]);
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

  it("treats template placeholders as Unknown", () => {
    const placeholder =
      "- Status: Proposed | Accepted | Implemented | Verified | Deprecated";
    expect(extractDocumentStatus(placeholder)).toBe("Unknown");
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

describe("extractFirstLineId", () => {
  it("returns identifier from markdown heading", () => {
    const id = extractFirstLineId("# FR-200 Sample Requirement\nBody");
    expect(id).toBe("FR-200");
  });

  it("strips byte order mark and whitespace", () => {
    const id = extractFirstLineId("\uFEFFT-123 Example Task\nDetails");
    expect(id).toBe("T-123");
  });

  it("returns null when identifier is absent", () => {
    expect(extractFirstLineId("# Heading Only")).toBeNull();
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
    expect(resolveLinkType("Related Requirements")).toBe("requirements");
    expect(resolveLinkType("Related Analyses")).toBe("analyses");
    expect(resolveLinkType("adr references")).toBe("adrs");
    expect(resolveLinkType("task items")).toBe("tasks");
    expect(resolveLinkType("Associated Design Document")).toBe("designs");
    expect(resolveLinkType("Associated Plan Document")).toBe("plans");
    expect(resolveLinkType("Prerequisite Requirements")).toBe("depends_on");
    expect(resolveLinkType("Dependent Requirements")).toBe("blocks");
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
    const result = parseArgs(["--check", "--write=output.md"]);
    expect(result).toEqual({
      checkMode: true,
      writePath: "output.md",
      showStatusDetails: false,
    });
  });

  it("supports bare --write", () => {
    const result = parseArgs(["--write"]);
    expect(result).toEqual({
      checkMode: false,
      writePath: "",
      showStatusDetails: false,
    });
  });

  it("enables status details when --status is provided", () => {
    const result = parseArgs(["--status"]);
    expect(result).toEqual({
      checkMode: false,
      writePath: null,
      showStatusDetails: true,
    });
  });
});

describe("traceability helpers", () => {
  it("calculates coverage and detects gaps", () => {
    const repoRoot = createTempDir();

    writeDoc(
      repoRoot,
      "docs/requirements/FR-0001-sample.md",
      requirementDoc({
        id: "FR-0001",
        title: "Sample Requirement",
        status: "Accepted",
        tasks: ["[T-0001-demo](../tasks/T-0001-demo/plan.md)"],
      }),
    );

    writeDoc(
      repoRoot,
      "docs/requirements/FR-0002-backlog.md",
      requirementDoc({
        id: "FR-0002",
        title: "Backlog Item",
        status: "Proposed",
        tasks: "N/A – Pending task definition",
      }),
    );

    writeDoc(
      repoRoot,
      "docs/analysis/AN-0001-investigation.md",
      analysisDoc({
        id: "AN-0001",
        title: "Investigation",
        status: "Complete",
        relatedRequirements: ["[FR-0001](../requirements/FR-0001-sample.md)"],
        relatedAdrs: ["[ADR-0001](../adr/ADR-0001-decision.md)"],
      }),
    );

    writeDoc(
      repoRoot,
      "docs/adr/ADR-0001-decision.md",
      adrDoc({
        id: "ADR-0001",
        title: "ADR-0001 Decision",
        status: "Accepted",
        impactedRequirements: ["[FR-0001](../requirements/FR-0001-sample.md)"],
        relatedTasks: ["[T-0001](../tasks/T-0001-demo/plan.md)"],
      }),
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-0001-demo/plan.md",
      taskPlanDoc({
        id: "T-0001",
        title: "Demo Task Implementation Plan",
        status: "Phase 1 In Progress",
        associatedDesign: "N/A – Awaiting design approval",
        requirements: ["[FR-0001](../../requirements/FR-0001-sample.md)"],
      }),
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-0002-unlinked/plan.md",
      taskPlanDoc({
        id: "T-0002",
        title: "Unlinked Plan",
        status: "Not Started",
        associatedDesign: "N/A – Pending design",
        requirements: "N/A – Requirements pending",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const coverage = calculateCoverage(documents);
    expect(coverage.total_requirements).toBe(2);
    expect(coverage.requirements_with_tasks).toBe(1);
    expect(coverage.coverage_percentage).toBeCloseTo(50);
    expect(coverage.total_tasks).toBe(2);
    expect(coverage.total_analyses).toBe(1);
    expect(coverage.total_adrs).toBe(1);

    expect(findImplementingTasks(documents, "FR-0001")).toEqual(["T-0001"]);
    expect(findOrphanRequirements(documents)).toEqual(["FR-0002"]);
    expect(findOrphanAdrs(documents)).toEqual([]);
    expect(findOrphanTasks(documents)).toEqual(["T-0002"]);

    const outputPath = join(repoRoot, "docs", "traceability.md");
    const markdown = renderTraceabilityMarkdown(documents, outputPath);
    expect(markdown).toContain("| Requirements | 2 |");
    expect(markdown).toContain("| Requirements with tasks | 1 (50%) |");
    expect(markdown).toContain(
      "| [FR-0001](requirements/FR-0001-sample.md) - FR-0001 Sample Requirement | Accepted |",
    );
    expect(markdown).toContain(
      "[T-0001](tasks/T-0001-demo/plan.md) (Phase 1 In Progress)",
    );
    expect(markdown).toContain(
      "- FR-0002: No upstream analysis or ADR references (Status: Proposed)",
    );
    expect(markdown).toContain(
      "- T-0002: No upstream analysis, requirement, or ADR references (Status: Not Started)",
    );
    expect(markdown).toContain("### Dependency Consistency");
    expect(markdown).toContain(
      "All prerequisite and dependent relationships are reciprocal with no contradictions or cycles detected.",
    );
  });

  it("merges links from task artifacts sharing the same ID", () => {
    const repoRoot = createTempDir();

    writeDoc(
      repoRoot,
      "docs/requirements/FR-5000-new-feature.md",
      requirementDoc({
        id: "FR-5000",
        title: "New Feature",
        status: "Accepted",
        tasks: ["[T-5000-example](../tasks/T-5000-example/plan.md)"],
      }),
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-5000-example/README.md",
      taskReadmeDoc({
        id: "T-5000",
        title: "T-5000 Example Task",
        status: "Proposed",
        planPath: "./plan.md",
        designPath: "./design.md",
      }),
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-5000-example/plan.md",
      taskPlanDoc({
        id: "T-5000",
        title: "Example Plan",
        status: "Phase 1 In Progress",
        associatedDesign: "[T-5000-design](./design.md)",
        requirements: ["[FR-5000](../../requirements/FR-5000-new-feature.md)"],
      }),
    );

    const documents = loadDocuments(repoRoot);
    const taskDoc = documents.get("T-5000");

    expect(taskDoc).toBeDefined();
    const mergedRequirements = [...new Set(taskDoc?.links.requirements ?? [])];
    expect(mergedRequirements).toEqual(["FR-5000"]);
    expect(taskDoc?.status).toBe("Proposed");
    expect(taskDoc?.metadataType).toBe("Task");
    expect(taskDoc?.path.endsWith("README.md")).toBe(true);

    expect(findImplementingTasks(documents, "FR-5000")).toEqual(["T-5000"]);
  });

  it("detects ADRs without analysis references", () => {
    const repoRoot = createTempDir();

    writeDoc(
      repoRoot,
      "docs/requirements/FR-7000-feature.md",
      requirementDoc({
        id: "FR-7000",
        title: "Feature Requirement",
        status: "Proposed",
        tasks: "N/A – Pending",
      }),
    );

    writeDoc(
      repoRoot,
      "docs/adr/ADR-7000-decision.md",
      adrDoc({
        id: "ADR-7000",
        title: "ADR-7000 Follow-up",
        status: "Proposed",
        impactedRequirements: ["[FR-7000](../requirements/FR-7000-feature.md)"],
        relatedTasks: "N/A – No tasks linked yet",
      }),
    );

    const documents = loadDocuments(repoRoot);
    expect(findOrphanAdrs(documents)).toEqual(["ADR-7000"]);
  });

  it("marks inferred dependencies and reports missing reciprocal links", () => {
    const repoRoot = createTempDir();

    writeDoc(
      repoRoot,
      "docs/requirements/FR-100-alpha.md",
      requirementDoc({
        id: "FR-100",
        title: "Alpha Requirement",
        status: "Proposed",
        prerequisites: "N/A – None documented",
        dependents: ["FR-200-beta"],
        tasks: "N/A – None",
      }),
    );

    writeDoc(
      repoRoot,
      "docs/requirements/FR-200-beta.md",
      requirementDoc({
        id: "FR-200",
        title: "Beta Requirement",
        status: "Proposed",
        prerequisites: "N/A – Pending documentation",
        dependents: "N/A – None",
        tasks: "N/A – None",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const outputPath = join(repoRoot, "docs", "traceability.md");
    const markdown = renderTraceabilityMarkdown(documents, outputPath);

    expect(markdown).toContain("(requirements/FR-100-alpha.md) (inferred)");
    expect(markdown).toContain(
      "add Prerequisite Requirements entry for [FR-100](requirements/FR-100-alpha.md) (inferred)",
    );
  });
});

describe("collectTaskReciprocalLinkIssues", () => {
  it("returns empty when tasks and related documents link both ways", () => {
    const repoRoot = createTempDir();

    const taskReadme = [
      "# T-4100 Example Task",
      "",
      "## Metadata",
      "",
      "- Type: Task",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Analyses:",
      "  - [AN-4100](../../analysis/AN-4100-example.md)",
      "- Related Requirements:",
      "  - [FR-4100](../../requirements/FR-4100-example.md)",
      "- Related ADRs:",
      "  - [ADR-4100](../../adr/ADR-4100-example.md)",
      "- Associated Design Document:",
      "  - N/A – None",
      "- Associated Plan Document:",
      "  - N/A – None",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/tasks/T-4100-example/README.md",
      `${taskReadme}\n## Summary\n\nReciprocal link validation.\n`,
    );

    const analysisDocContent = [
      "# AN-4100 Example Analysis",
      "",
      "## Metadata",
      "",
      "- Type: Analysis",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks:",
      "  - [T-4100](../tasks/T-4100-example/README.md)",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/analysis/AN-4100-example.md",
      `${analysisDocContent}\n## Summary\n\nConsistency check.\n`,
    );

    const requirementDocContent = [
      "# FR-4100 Example Requirement",
      "",
      "## Metadata",
      "",
      "- Type: Functional Requirement",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks:",
      "  - [T-4100](../tasks/T-4100-example/README.md)",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/requirements/FR-4100-example.md",
      `${requirementDocContent}\n## Requirement Statement\n\nTwo-way links.\n`,
    );

    const adrDocContent = [
      "# ADR-4100 Example Decision",
      "",
      "## Metadata",
      "",
      "- Type: ADR",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks:",
      "  - [T-4100](../tasks/T-4100-example/README.md)",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/adr/ADR-4100-example.md",
      `${adrDocContent}\n## Context\n\nBidirectional traceability maintained.\n`,
    );

    const documents = loadDocuments(repoRoot);
    expect(collectTaskReciprocalLinkIssues(documents)).toEqual([]);
  });

  it("reports missing reciprocal links and documents", () => {
    const repoRoot = createTempDir();

    const taskReadme = [
      "# T-5100 Missing Reciprocity",
      "",
      "## Metadata",
      "",
      "- Type: Task",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Analyses:",
      "  - [AN-5100](../../analysis/AN-5100-example.md)",
      "- Related Requirements:",
      "  - [FR-5100](../../requirements/FR-5100-example.md)",
      "- Related ADRs:",
      "  - [ADR-5100](../../adr/ADR-5100-example.md)",
      "- Associated Design Document:",
      "  - N/A – None",
      "- Associated Plan Document:",
      "  - N/A – None",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/tasks/T-5100-example/README.md",
      `${taskReadme}\n## Summary\n\nMissing reciprocal entries.\n`,
    );

    const requirementDocContent = [
      "# FR-5100 Example Requirement",
      "",
      "## Metadata",
      "",
      "- Type: Functional Requirement",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks: N/A – None",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/requirements/FR-5100-example.md",
      `${requirementDocContent}\n## Requirement Statement\n\nPlaceholder.\n`,
    );

    const adrDocContent = [
      "# ADR-5100 Example Decision",
      "",
      "## Metadata",
      "",
      "- Type: ADR",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks: N/A – None linked yet",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/adr/ADR-5100-example.md",
      `${adrDocContent}\n## Context\n\nPlaceholder.\n`,
    );

    const documents = loadDocuments(repoRoot);
    const issues = collectTaskReciprocalLinkIssues(documents);
    expect(issues).toHaveLength(1);
    expect(issues[0]?.taskId).toBe("T-5100");
    expect(issues[0]?.messages).toEqual([
      "ADR ADR-5100 does not list T-5100 under Related Tasks",
      "Analysis AN-5100 is missing",
      "Requirement FR-5100 does not list T-5100 under Related Tasks",
    ]);
  });
});

describe("collectTaskDesignPlanIssues", () => {
  it("returns empty when task links are reciprocal", () => {
    const repoRoot = createTempDir();

    writeDoc(
      repoRoot,
      "docs/tasks/T-1100-example/README.md",
      taskReadmeDoc({
        id: "T-1100",
        title: "Example Task",
        status: "Draft",
        planPath: "./plan.md",
        designPath: "./design.md",
      }),
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-1100-example/plan.md",
      taskPlanDoc({
        id: "T-1100",
        title: "Example Plan",
        status: "Draft",
        associatedDesign: "[T-1100-design](./design.md)",
        requirements: "N/A – Pending",
      }),
    );

    writeDoc(
      repoRoot,
      "docs/tasks/T-1100-example/design.md",
      taskDesignDoc({
        id: "T-1100",
        title: "Example Design",
        status: "Draft",
        associatedPlan: "[T-1100-plan](./plan.md)",
      }),
    );

    const documents = loadDocuments(repoRoot);
    expect(collectTaskDesignPlanIssues(documents)).toEqual([]);
  });

  it("flags designs referenced by a task that do not define plans", () => {
    const repoRoot = createTempDir();

    const readme = [
      "# T-2100 Missing Plan",
      "",
      "## Metadata",
      "",
      "- Type: Task",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Associated Design Document:",
      "  - [T-2100-design](./design.md)",
      "",
    ].join("\n");

    writeDoc(repoRoot, "docs/tasks/T-2100-missing-plan/README.md", readme);

    writeDoc(
      repoRoot,
      "docs/tasks/T-2100-missing-plan/design.md",
      taskDesignDoc({
        id: "T-2100",
        title: "Missing Plan Design",
        status: "Draft",
        associatedPlan: "N/A – Plan pending",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const issues = collectTaskDesignPlanIssues(documents);
    expect(issues).toHaveLength(1);
    expect(issues[0]?.taskId).toBe("T-2100");
    expect(issues[0]?.messages).toContain(
      "Design T-2100 referenced from T-2100 does not define any plan documents",
    );
  });

  it("flags plans referenced by a task that do not define designs", () => {
    const repoRoot = createTempDir();

    const readme = [
      "# T-3100 Missing Design",
      "",
      "## Metadata",
      "",
      "- Type: Task",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Associated Plan Document:",
      "  - [T-3100-plan](./plan.md)",
      "",
    ].join("\n");

    writeDoc(repoRoot, "docs/tasks/T-3100-missing-design/README.md", readme);

    writeDoc(
      repoRoot,
      "docs/tasks/T-3100-missing-design/plan.md",
      taskPlanDoc({
        id: "T-3100",
        title: "Missing Design Plan",
        status: "Draft",
        associatedDesign: "N/A – Pending design",
        requirements: "N/A – Pending",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const issues = collectTaskDesignPlanIssues(documents);
    expect(issues).toHaveLength(1);
    expect(issues[0]?.taskId).toBe("T-3100");
    expect(issues[0]?.messages).toContain(
      "Plan T-3100 referenced from T-3100 does not define any design documents",
    );
  });
});

describe("printStatus", () => {
  const originalLog = console.log;
  const originalError = console.error;
  let logCalls: string[];

  beforeEach(() => {
    logCalls = [];
    console.log = (...args: unknown[]) => {
      logCalls.push(args.map(String).join(" "));
    };
    console.error = (...args: unknown[]) => {
      logCalls.push(args.map(String).join(" "));
    };
  });

  afterEach(() => {
    console.log = originalLog;
    console.error = originalError;
  });

  it("prints summary and gaps by default", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-1000-missing-task.md",
      requirementDoc({
        id: "FR-1000",
        title: "Missing Task",
        status: "Proposed",
        tasks: "N/A – Not yet planned",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/tasks/T-2000-unlinked/plan.md",
      taskPlanDoc({
        id: "T-2000",
        title: "Unlinked Plan",
        status: "Not Started",
        associatedDesign: "N/A – Pending design",
        requirements: "N/A – Requirements pending",
      }),
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, false);

    const output = logCalls.join("\n");
    expect(output).toContain("=== Kopi TDL Status ===");
    expect(output).not.toContain("Coverage:");
    expect(output).toContain("Gaps:");
    expect(output).toContain("FR-1000");
    expect(output).toContain("T-2000");
    expect(output).not.toContain("Status by Document Type:");
    expect(output).toContain("✓ Dependency links consistent");
    expect(output).toContain("✓ Task reciprocal links consistent");
    expect(output).toContain("✓ Document ID headings consistent");
  });

  it("includes coverage and status breakdown when showStatusDetails=true", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-1100-linked.md",
      requirementDoc({
        id: "FR-1100",
        title: "Linked Requirement",
        status: "Accepted",
        tasks: ["[T-1100](../tasks/T-1100-linked/plan.md)"],
      }),
    );
    writeDoc(
      repoRoot,
      "docs/tasks/T-1100-linked/plan.md",
      taskPlanDoc({
        id: "T-1100",
        title: "Linked Task",
        status: "In Progress",
        associatedDesign: "N/A – Pending design",
        requirements: ["[FR-1100](../../requirements/FR-1100-linked.md)"],
      }),
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, true);

    const output = logCalls.join("\n");
    expect(output).toContain("Coverage:");
    expect(output).toContain("Status by Document Type:");
    expect(output).toContain("  Requirements:");
    expect(output).toContain("  Tasks:");
  });

  it("reports dependency consistency issues when reciprocal links are missing", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-400-alpha.md",
      requirementDoc({
        id: "FR-400",
        title: "Alpha",
        status: "Proposed",
        prerequisites: "N/A – None",
        dependents: ["FR-401-beta"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-401-beta.md",
      requirementDoc({
        id: "FR-401",
        title: "Beta",
        status: "Proposed",
        prerequisites: "N/A – Pending",
        dependents: "N/A – None",
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/tasks/T-400-sync/plan.md",
      taskPlanDoc({
        id: "T-400",
        title: "Sync Plan",
        status: "Phase 1 In Progress",
        associatedDesign: "N/A – Pending design",
        requirements: [
          "[FR-400](../../requirements/FR-400-alpha.md)",
          "[FR-401](../../requirements/FR-401-beta.md)",
        ],
      }),
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, false);

    const output = logCalls.join("\n");
    expect(output).toContain("Dependency consistency issues:");
    expect(output).toContain("Missing prerequisite link(s) for FR-400");
  });

  it("reports task reciprocity issues when related documents do not link back", () => {
    const repoRoot = createTempDir();

    const readme = [
      "# T-5200 Reciprocity Regression",
      "",
      "## Metadata",
      "",
      "- Type: Task",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Requirements:",
      "  - [FR-5200](../../requirements/FR-5200-reciprocity.md)",
      "- Related Analyses: N/A – None",
      "- Related ADRs: N/A – None",
      "",
    ].join("\n");

    writeDoc(repoRoot, "docs/tasks/T-5200-example/README.md", readme);

    const requirementContent = [
      "# FR-5200 Reciprocity Check",
      "",
      "## Metadata",
      "",
      "- Type: Functional Requirement",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks: N/A – Pending linkage",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/requirements/FR-5200-reciprocity.md",
      `${requirementContent}\n## Requirement Statement\n\nEnsure reciprocity is enforced.\n`,
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, false);

    const output = logCalls.join("\n");
    expect(output).toContain("Task reciprocity issues:");
    expect(output).toContain(
      "Requirement FR-5200 does not list T-5200 under Related Tasks",
    );
  });

  it("reports mutual prerequisite and dependent contradictions", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-900-loop-a.md",
      requirementDoc({
        id: "FR-900",
        title: "Loop A",
        status: "Accepted",
        prerequisites: ["[FR-901](../requirements/FR-901-loop-b.md)"],
        dependents: ["[FR-901](../requirements/FR-901-loop-b.md)"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-901-loop-b.md",
      requirementDoc({
        id: "FR-901",
        title: "Loop B",
        status: "Accepted",
        prerequisites: ["[FR-900](../requirements/FR-900-loop-a.md)"],
        dependents: ["[FR-900](../requirements/FR-900-loop-a.md)"],
        tasks: "N/A – None",
      }),
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, false);

    const output = logCalls.join("\n");
    expect(output).toContain("Dependency consistency issues:");
    expect(output).toContain(
      "FR-900 and FR-901 list each other as prerequisites; remove the contradiction.",
    );
    expect(output).toContain(
      "FR-900 and FR-901 list each other as dependents; remove the contradiction.",
    );
  });

  it("reports prerequisite cycles spanning three requirements", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-910-cycle-a.md",
      requirementDoc({
        id: "FR-910",
        title: "Cycle A",
        status: "Accepted",
        prerequisites: ["[FR-920](../requirements/FR-920-cycle-b.md)"],
        dependents: ["[FR-930](../requirements/FR-930-cycle-c.md)"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-920-cycle-b.md",
      requirementDoc({
        id: "FR-920",
        title: "Cycle B",
        status: "Accepted",
        prerequisites: ["[FR-930](../requirements/FR-930-cycle-c.md)"],
        dependents: ["[FR-910](../requirements/FR-910-cycle-a.md)"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-930-cycle-c.md",
      requirementDoc({
        id: "FR-930",
        title: "Cycle C",
        status: "Accepted",
        prerequisites: ["[FR-910](../requirements/FR-910-cycle-a.md)"],
        dependents: ["[FR-920](../requirements/FR-920-cycle-b.md)"],
        tasks: "N/A – None",
      }),
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, false);

    const output = logCalls.join("\n");
    expect(output).toContain("Dependency consistency issues:");
    expect(output).toContain(
      "Prerequisite cycle detected among: FR-910 -> FR-920 -> FR-930",
    );
    expect(output).not.toContain("Missing prerequisite link(s)");
    expect(output).not.toContain("Missing dependent link(s)");
  });

  it("reports heading mismatches in status output", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-500-heading-mismatch.md",
      requirementDoc({
        id: "FR-501",
        title: "Heading Mismatch",
        status: "Proposed",
        tasks: "N/A – Not yet planned",
      }),
    );

    const documents = loadDocuments(repoRoot);
    printStatus(documents, false);

    const output = logCalls.join("\n");
    expect(output).toContain("Document ID heading mismatches detected:");
    expect(output).toContain("expected FR-500 on first line, found FR-501");
  });
});

describe("findHeadingMismatches", () => {
  it("detects mismatched heading identifiers", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-700-mismatch.md",
      requirementDoc({
        id: "FR-701",
        title: "Mismatch",
        status: "Proposed",
        prerequisites: "N/A – None",
        dependents: "N/A – None",
        tasks: "N/A – Not yet planned",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const mismatches = findHeadingMismatches(documents);

    expect(mismatches).toHaveLength(1);
    const mismatch = mismatches[0];
    expect(mismatch.expectedId).toBe("FR-700");
    expect(mismatch.actualId).toBe("FR-701");
    expect(mismatch.path).toBe(
      join(repoRoot, "docs/requirements/FR-700-mismatch.md"),
    );
  });

  it("returns empty array when headings match document identifiers", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-710-aligned.md",
      requirementDoc({
        id: "FR-710",
        title: "Aligned",
        status: "Proposed",
      }),
    );

    const documents = loadDocuments(repoRoot);
    expect(findHeadingMismatches(documents)).toEqual([]);
  });
});

describe("checkIntegrity", () => {
  const originalError = console.error;

  afterEach(() => {
    console.error = originalError;
  });

  it("returns false when prerequisite links are missing", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-600-alpha.md",
      requirementDoc({
        id: "FR-600",
        title: "Alpha",
        status: "Proposed",
        prerequisites: "N/A – None",
        dependents: ["FR-601-beta"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-601-beta.md",
      requirementDoc({
        id: "FR-601",
        title: "Beta",
        status: "Proposed",
        prerequisites: "N/A – Pending",
        dependents: "N/A – None",
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/tasks/T-600-plan/plan.md",
      taskPlanDoc({
        id: "T-600",
        title: "Plan",
        status: "Phase 1 In Progress",
        associatedDesign: "N/A – Pending design",
        requirements: [
          "[FR-600](../../requirements/FR-600-alpha.md)",
          "[FR-601](../../requirements/FR-601-beta.md)",
        ],
      }),
    );

    const documents = loadDocuments(repoRoot);
    const errors: unknown[][] = [];
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };

    const result = checkIntegrity(documents);
    expect(result).toBe(false);
    const flattened = errors.map((entry) => entry.join(" ")).join("\n");
    expect(flattened).toContain("Missing prerequisite link(s) for FR-600");
  });

  it("returns false when tasks reference documents without reciprocal links", () => {
    const repoRoot = createTempDir();

    const taskReadme = [
      "# T-7200 Reciprocity Failure",
      "",
      "## Metadata",
      "",
      "- Type: Task",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Requirements:",
      "  - [FR-7200](../../requirements/FR-7200-example.md)",
      "- Related Analyses: N/A – None",
      "- Related ADRs: N/A – None",
      "",
    ].join("\n");

    writeDoc(repoRoot, "docs/tasks/T-7200-example/README.md", taskReadme);

    const requirement = [
      "# FR-7200 Example Requirement",
      "",
      "## Metadata",
      "",
      "- Type: Functional Requirement",
      "- Status: Draft",
      "",
      "## Links",
      "",
      "- Related Tasks: N/A – Pending linkage",
      "",
    ].join("\n");

    writeDoc(
      repoRoot,
      "docs/requirements/FR-7200-example.md",
      `${requirement}\n## Requirement Statement\n\nAwaiting reciprocity.\n`,
    );

    const documents = loadDocuments(repoRoot);
    const errors: unknown[][] = [];
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };

    const result = checkIntegrity(documents);
    expect(result).toBe(false);
    const flattened = errors.map((entry) => entry.join(" ")).join("\n");
    expect(flattened).toContain("Task reciprocity issues detected:");
    expect(flattened).toContain(
      "Requirement FR-7200 does not list T-7200 under Related Tasks",
    );
  });

  it("returns false when requirements list each other as prerequisites", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-9010-loop-a.md",
      requirementDoc({
        id: "FR-9010",
        title: "Loop A",
        status: "Accepted",
        prerequisites: ["[FR-9011](../requirements/FR-9011-loop-b.md)"],
        dependents: ["[FR-9011](../requirements/FR-9011-loop-b.md)"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-9011-loop-b.md",
      requirementDoc({
        id: "FR-9011",
        title: "Loop B",
        status: "Accepted",
        prerequisites: ["[FR-9010](../requirements/FR-9010-loop-a.md)"],
        dependents: ["[FR-9010](../requirements/FR-9010-loop-a.md)"],
        tasks: "N/A – None",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const errors: unknown[][] = [];
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };

    const result = checkIntegrity(documents);
    expect(result).toBe(false);
    const flattened = errors.map((entry) => entry.join(" ")).join("\n");
    expect(flattened).toContain(
      "FR-9010 and FR-9011 list each other as prerequisites; remove the contradiction.",
    );
    expect(flattened).toContain(
      "FR-9010 and FR-9011 list each other as dependents; remove the contradiction.",
    );
  });

  it("returns false when prerequisite cycles are present", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-9020-cycle-a.md",
      requirementDoc({
        id: "FR-9020",
        title: "Cycle A",
        status: "Accepted",
        prerequisites: ["[FR-9021](../requirements/FR-9021-cycle-b.md)"],
        dependents: ["[FR-9022](../requirements/FR-9022-cycle-c.md)"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-9021-cycle-b.md",
      requirementDoc({
        id: "FR-9021",
        title: "Cycle B",
        status: "Accepted",
        prerequisites: ["[FR-9022](../requirements/FR-9022-cycle-c.md)"],
        dependents: ["[FR-9020](../requirements/FR-9020-cycle-a.md)"],
        tasks: "N/A – None",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/requirements/FR-9022-cycle-c.md",
      requirementDoc({
        id: "FR-9022",
        title: "Cycle C",
        status: "Accepted",
        prerequisites: ["[FR-9020](../requirements/FR-9020-cycle-a.md)"],
        dependents: ["[FR-9021](../requirements/FR-9021-cycle-b.md)"],
        tasks: "N/A – None",
      }),
    );

    const documents = loadDocuments(repoRoot);
    const errors: unknown[][] = [];
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };

    const result = checkIntegrity(documents);
    expect(result).toBe(false);
    const flattened = errors.map((entry) => entry.join(" ")).join("\n");
    expect(flattened).toContain(
      "Prerequisite cycle detected among: FR-9020 -> FR-9021 -> FR-9022",
    );
    expect(flattened).not.toContain("Missing prerequisite link(s)");
    expect(flattened).not.toContain("Missing dependent link(s)");
  });

  it("reports heading mismatches as integrity failures", () => {
    const repoRoot = createTempDir();
    writeDoc(
      repoRoot,
      "docs/requirements/FR-800-mismatch.md",
      requirementDoc({
        id: "FR-801",
        title: "Mismatch",
        status: "Proposed",
        prerequisites: "N/A – None",
        dependents: "N/A – None",
        tasks: "N/A – Not yet planned",
      }),
    );
    writeDoc(
      repoRoot,
      "docs/analysis/AN-900-alignment.md",
      analysisDoc({
        id: "AN-900",
        title: "Alignment",
        status: "Complete",
        relatedRequirements: ["[FR-800](../requirements/FR-800-mismatch.md)"],
      }),
    );

    const documents = loadDocuments(repoRoot);
    const errors: unknown[][] = [];
    console.error = (...args: unknown[]) => {
      errors.push(args);
    };

    const result = checkIntegrity(documents);
    expect(result).toBe(false);
    const output = errors.map((entry) => entry.join(" ")).join("\n");
    expect(output).toContain("Document ID heading mismatches detected");
    expect(output).toContain("expected FR-800 on first line, found FR-801");
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
      requirementDoc({
        id: "FR-0001",
        title: "Ready Requirement",
        status: "Implemented",
        tasks: ["[T-0001-ready](../tasks/T-0001-ready/plan.md)"],
      }),
    );
    writeDoc(
      repo,
      "docs/analysis/AN-0001-seed.md",
      analysisDoc({
        id: "AN-0001",
        title: "Seed",
        status: "Complete",
        relatedRequirements: ["[FR-0001](../requirements/FR-0001-ready.md)"],
        relatedAdrs: "N/A – None",
        relatedAnalyses: "N/A – No previous analysis",
      }),
    );
    writeDoc(
      repo,
      "docs/tasks/T-0001-ready/plan.md",
      taskPlanDoc({
        id: "T-0001",
        title: "Ready Plan",
        status: "Completed",
        associatedDesign: "N/A – Design complete",
        requirements: ["[FR-0001](../../requirements/FR-0001-ready.md)"],
      }),
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
      requirementDoc({
        id: "FR-0002",
        title: "Gap Requirement",
        status: "Proposed",
        tasks: "N/A – Pending",
      }),
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
