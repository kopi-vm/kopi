#!/usr/bin/env bun

/**
 * Display TDL traceability status by parsing Links sections in documents.
 * Mirrors the previous Python implementation with equivalent behaviour.
 */

import type { Dirent } from "node:fs";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import process from "node:process";

type DocumentType = "analysis" | "requirement" | "adr" | "task" | "unknown";
type LinkMap = Record<string, string[]>;

type CoverageReport = {
  total_requirements: number;
  total_tasks: number;
  total_analyses: number;
  total_adrs: number;
  requirements_with_tasks: number;
  coverage_percentage: number;
};

type DocSource = {
  baseDir: string;
  recursive: boolean;
  match: (relativePath: string) => boolean;
};

class TDLDocument {
  readonly path: string;
  readonly filename: string;
  readonly docId: string;
  readonly docType: DocumentType;
  readonly links: LinkMap;
  readonly status: string;

  constructor(filePath: string) {
    this.path = filePath;
    this.filename = filePath.split(/[/\\]/).pop() ?? filePath;
    this.docId = TDLDocument.extractId(this.filename);
    this.docType = TDLDocument.extractType(this.filename, filePath);
    this.links = TDLDocument.parseLinks(filePath);
    this.status = TDLDocument.extractStatus(filePath);
  }

  private static extractId(filename: string): string {
    const match = filename.match(/^([A-Z]+-[^-]+)/);
    return match ? match[1] : filename;
  }

  private static extractType(filename: string, filePath: string): DocumentType {
    if (filename.startsWith("AN-")) return "analysis";
    if (filename.startsWith("FR-")) return "requirement";
    if (filename.startsWith("NFR-")) return "requirement";
    if (filename.startsWith("ADR-")) return "adr";
    if (filename.startsWith("T-")) return "task";
    // Fallback to directory-based inference for task documents (plan/design files)
    if (/\bdocs[/\\]tasks\b/.test(filePath)) return "task";
    return "unknown";
  }

  private static parseLinks(filePath: string): LinkMap {
    const links: LinkMap = {};
    const content = safeReadFile(filePath);
    if (content === null) return links;

    const linksMatch = content.match(/## Links\s*\n([\s\S]*?)(?=\n##|$)/);
    if (!linksMatch) return links;

    const linksContent = linksMatch[1];
    const lines = linksContent.split(/\r?\n/);
    for (const rawLine of lines) {
      const line = rawLine.trim();
      if (!line.startsWith("- ")) continue;

      const colonIndex = line.indexOf(":");
      if (colonIndex === -1) continue;

      const label = line.slice(2, colonIndex).trim().toLowerCase();
      const value = line.slice(colonIndex + 1);
      const linkType = resolveLinkType(label);
      if (!linkType) continue;

      const ids = extractIds(value);
      if (ids.length === 0) continue;

      links[linkType] = links[linkType] ?? [];
      links[linkType].push(...ids);
    }

    return links;
  }

  private static extractStatus(filePath: string): string {
    const content = safeReadFile(filePath);
    if (content === null) return "Unknown";

    const statusMatch = content.match(/^\s*-\s*Status:\s*(.+)$/m);
    if (statusMatch) return statusMatch[1].trim();

    return "Unknown";
  }
}

class TraceabilityAnalyzer {
  private readonly repoRoot: string;
  private readonly documents: Map<string, TDLDocument> = new Map();

  constructor(repoRoot: string) {
    this.repoRoot = repoRoot;
    this.loadDocuments();
  }

  private loadDocuments(): void {
    const sources: DocSource[] = [
      {
        baseDir: join(this.repoRoot, "docs", "analysis"),
        recursive: false,
        match: (p) => p.endsWith(".md"),
      },
      {
        baseDir: join(this.repoRoot, "docs", "requirements"),
        recursive: false,
        match: (p) => p.endsWith(".md"),
      },
      {
        baseDir: join(this.repoRoot, "docs", "adr"),
        recursive: false,
        match: (p) => p.endsWith(".md"),
      },
      {
        baseDir: join(this.repoRoot, "docs", "tasks"),
        recursive: true,
        match: (p) => p.endsWith("plan.md"),
      },
      {
        baseDir: join(this.repoRoot, "docs", "tasks"),
        recursive: true,
        match: (p) => p.endsWith("design.md"),
      },
    ];

    for (const source of sources) {
      if (!existsSync(source.baseDir)) continue;
      for (const filePath of walkFiles(source.baseDir, source.recursive)) {
        if (!source.match(filePath)) continue;
        if (filePath.includes("traceability.md")) continue;
        if (
          filePath.includes(`docs${sep}templates${sep}`) ||
          filePath.includes("docs/templates/")
        )
          continue;

        const doc = new TDLDocument(filePath);
        this.documents.set(doc.docId, doc);
      }
    }
  }

  private requirementDocs(): TDLDocument[] {
    return [...this.documents.values()].filter(
      (doc) => doc.docType === "requirement",
    );
  }

  private taskDocs(): TDLDocument[] {
    return [...this.documents.values()].filter((doc) => doc.docType === "task");
  }

  findImplementingTasks(reqId: string): string[] {
    const tasks: string[] = [];
    for (const doc of this.taskDocs()) {
      const linked = doc.links.requirements ?? [];
      if (linked.includes(reqId)) tasks.push(doc.docId);
    }
    return tasks;
  }

  findOrphanRequirements(): string[] {
    const orphans: string[] = [];
    for (const doc of this.requirementDocs()) {
      if (this.findImplementingTasks(doc.docId).length === 0) {
        orphans.push(doc.docId);
      }
    }
    return orphans;
  }

  findOrphanTasks(): string[] {
    const orphans: string[] = [];
    for (const doc of this.taskDocs()) {
      if ((doc.links.requirements ?? []).length === 0) {
        orphans.push(doc.docId);
      }
    }
    return orphans;
  }

  calculateCoverage(): CoverageReport {
    const requirements = this.requirementDocs();
    const tasks = this.taskDocs();
    const analyses = [...this.documents.values()].filter(
      (doc) => doc.docType === "analysis",
    );
    const adrs = [...this.documents.values()].filter(
      (doc) => doc.docType === "adr",
    );

    const requirementsWithTasks = requirements.filter(
      (req) => this.findImplementingTasks(req.docId).length > 0,
    ).length;
    const coveragePercentage = requirements.length
      ? (requirementsWithTasks / requirements.length) * 100
      : 0;

    return {
      total_requirements: requirements.length,
      total_tasks: tasks.length,
      total_analyses: analyses.length,
      total_adrs: adrs.length,
      requirements_with_tasks: requirementsWithTasks,
      coverage_percentage: coveragePercentage,
    };
  }

  printStatus(gapsOnly: boolean): void {
    if (!gapsOnly) {
      console.log("=== Kopi TDL Status ===\n");
      const coverage = this.calculateCoverage();
      console.log("Coverage:");
      console.log(
        `  Documents: ${coverage.total_analyses} analyses, ${coverage.total_requirements} requirements, ` +
          `${coverage.total_adrs} ADRs, ${coverage.total_tasks} tasks`,
      );
      console.log(
        `  Implementation: ${coverage.requirements_with_tasks}/${coverage.total_requirements} requirements have tasks ` +
          `(${coverage.coverage_percentage.toFixed(0)}%)`,
      );
      console.log();
    }

    const orphanRequirements = this.findOrphanRequirements();
    const orphanTasks = this.findOrphanTasks();

    if (orphanRequirements.length || orphanTasks.length) {
      console.log("Gaps:");
      for (const reqId of orphanRequirements.sort()) {
        const doc = this.documents.get(reqId);
        const status = doc?.status ?? "Unknown";
        console.log(`  ⚠ ${reqId}: No implementing task (Status: ${status})`);
      }
      for (const taskId of orphanTasks.sort()) {
        console.log(`  ⚠ ${taskId}: No linked requirements`);
      }
      console.log();
    } else if (!gapsOnly) {
      console.log("✓ No gaps detected\n");
    }

    if (!gapsOnly) {
      console.log("Status by Document Type:");
      const byType = new Map<DocumentType, TDLDocument[]>();
      for (const doc of this.documents.values()) {
        let bucket = byType.get(doc.docType);
        if (!bucket) {
          bucket = [];
          byType.set(doc.docType, bucket);
        }
        bucket.push(doc);
      }

      for (const docType of [
        "analysis",
        "requirement",
        "adr",
        "task",
      ] as const) {
        if (!byType.has(docType)) continue;
        const docs = byType.get(docType);
        if (!docs) continue;
        console.log(`\n  ${capitalize(docType)}s:`);
        const byStatus = new Map<string, number>();
        for (const doc of docs) {
          const status = doc.status;
          byStatus.set(status, (byStatus.get(status) ?? 0) + 1);
        }
        for (const [status, count] of [...byStatus.entries()].sort((a, b) =>
          a[0].localeCompare(b[0]),
        )) {
          console.log(`    ${status}: ${count}`);
        }
      }
    }
  }

  checkIntegrity(): boolean {
    const orphanRequirements = this.findOrphanRequirements();
    const orphanTasks = this.findOrphanTasks();

    if (orphanRequirements.length || orphanTasks.length) {
      console.error("Traceability gaps detected:");
      for (const reqId of orphanRequirements) {
        console.error(`  - ${reqId}: No implementing task`);
      }
      for (const taskId of orphanTasks) {
        console.error(`  - ${taskId}: No linked requirements`);
      }
      return false;
    }

    return true;
  }
}

function safeReadFile(path: string): string | null {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    console.error(
      `Warning: Failed to read ${path}: ${(error as Error).message}`,
    );
    return null;
  }
}

function* walkFiles(rootDir: string, recursive: boolean): Generator<string> {
  const stack: string[] = [rootDir];
  while (stack.length) {
    const current = stack.pop();
    if (!current) continue;
    let entries: Dirent[];
    try {
      entries = readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const entryPath = join(current, entry.name);
      if (entry.isDirectory()) {
        if (recursive) stack.push(entryPath);
      } else if (entry.isFile()) {
        yield entryPath;
      }
    }
  }
}

function resolveLinkType(label: string): string | null {
  if (label.includes("formal requirement") || label.includes("requirements"))
    return "requirements";
  if (label.includes("related adr")) return "adrs";
  if (label.includes("related analyse") || label.includes("related analysis"))
    return "analyses";
  if (label === "design") return "design";
  if (label === "plan") return "plan";
  return null;
}

function extractIds(value: string): string[] {
  const matches = value.match(
    /[A-Z]+-[0-9a-z]{4,5}(?:-[^/\s\]]+)?|[A-Z]+-\d+/g,
  );
  return matches ? matches : [];
}

function capitalize(value: string): string {
  if (!value) return value;
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function findRepoRoot(startDir: string): string | null {
  let current = resolve(startDir);
  while (true) {
    if (
      existsSync(join(current, ".git")) ||
      existsSync(join(current, "Cargo.toml"))
    ) {
      return current;
    }
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
  return null;
}

function parseArgs(argv: string[]): { gapsOnly: boolean; checkMode: boolean } {
  let gapsOnly = false;
  let checkMode = false;
  for (const arg of argv) {
    if (arg === "--gaps") {
      gapsOnly = true;
    } else if (arg === "--check") {
      checkMode = true;
    } else {
      console.error(`Unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  return { gapsOnly, checkMode };
}

function main(): number {
  const { gapsOnly, checkMode } = parseArgs(process.argv.slice(2));
  const repoRoot = findRepoRoot(process.cwd());
  if (!repoRoot) {
    console.error("Error: Could not find repository root");
    return 1;
  }

  const analyzer = new TraceabilityAnalyzer(repoRoot);

  if (checkMode) {
    if (!analyzer.checkIntegrity()) {
      return 1;
    }
    console.log("✓ Traceability check passed");
    return 0;
  }

  analyzer.printStatus(gapsOnly);
  return 0;
}

if (import.meta.main) {
  process.exit(main());
}
