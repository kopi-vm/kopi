#!/usr/bin/env bun

/**
 * Display TDL traceability status by parsing Links sections in documents.
 * Mirrors the previous Python implementation with equivalent behaviour.
 */

import type { Dirent } from "node:fs";
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import process from "node:process";

export type DocumentType =
  | "analysis"
  | "requirement"
  | "adr"
  | "task"
  | "unknown";
type LinkMap = Record<string, string[]>;

export type CoverageReport = {
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

export type TDLDocument = {
  readonly path: string;
  readonly filename: string;
  readonly docId: string;
  readonly docType: DocumentType;
  readonly links: LinkMap;
  readonly status: string;
  readonly title: string;
};

export function makeTDLDocument(filePath: string): TDLDocument {
  const filename = filePath.split(/[/\\]/).pop() ?? filePath;
  const content = safeReadFile(filePath);
  return {
    path: filePath,
    filename,
    docId: extractDocumentId(filename, filePath, content),
    docType: inferDocumentType(filename, filePath),
    links: parseDocumentLinks(content),
    status: extractDocumentStatus(content),
    title: extractDocumentTitle(content),
  };
}

export function extractDocumentId(
  filename: string,
  filePath: string,
  content: string | null,
): string {
  const directMatch = filename.match(/^([A-Z]+-[^-]+)/);
  if (directMatch) return directMatch[1];

  const pathMatch = filePath.match(/([A-Z]+-[0-9a-z]+)(?=[-./\\]|$)/);
  if (pathMatch) return pathMatch[1];

  if (content) {
    const metadataMatch = content.match(/^\s*-\s*ID:\s*([A-Z]+-[^\s]+)/im);
    if (metadataMatch) return metadataMatch[1];
  }

  return filename;
}

export function inferDocumentType(
  filename: string,
  filePath: string,
): DocumentType {
  if (filename.startsWith("AN-")) return "analysis";
  if (filename.startsWith("FR-")) return "requirement";
  if (filename.startsWith("NFR-")) return "requirement";
  if (filename.startsWith("ADR-")) return "adr";
  if (filename.startsWith("T-")) return "task";
  const normalizedPath = filePath.replace(/\\/g, "/");
  if (normalizedPath.includes("docs/tasks/T-")) return "task";
  return "unknown";
}

export function parseDocumentLinks(content: string | null): LinkMap {
  const links: LinkMap = {};
  if (content === null) return links;

  const linksMatch = content.match(/## Links\s*\n([\s\S]*?)(?=\n##|$)/);
  if (!linksMatch) return links;

  const linksContent = linksMatch[1];
  const lines = linksContent.split(/\r?\n/);
  let currentLinkType: string | null = null;
  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line) {
      currentLinkType = null;
      continue;
    }
    if (!line.startsWith("- ")) continue;

    const colonIndex = line.indexOf(":");
    if (colonIndex !== -1) {
      const label = line.slice(2, colonIndex).trim().toLowerCase();
      const value = line.slice(colonIndex + 1);
      const linkType = resolveLinkType(label);
      currentLinkType = linkType;
      if (!linkType) continue;

      const ids = extractIds(value);
      if (ids.length === 0) continue;

      links[linkType] = links[linkType] ?? [];
      links[linkType].push(...ids);
      continue;
    }

    if (!currentLinkType) continue;
    const ids = extractIds(line.slice(2));
    if (ids.length === 0) continue;
    links[currentLinkType] = links[currentLinkType] ?? [];
    links[currentLinkType].push(...ids);
  }

  return links;
}

export function extractDocumentStatus(content: string | null): string {
  if (content === null) return "Unknown";

  const statusMatch = content.match(/^\s*-\s*Status:\s*(.+)$/m);
  if (statusMatch) return statusMatch[1].trim();

  return "Unknown";
}

export function extractDocumentTitle(content: string | null): string {
  if (content === null) return "";
  const match = content.match(/^#\s+(.+)$/m);
  return match ? match[1].trim() : "";
}

export function loadDocuments(repoRoot: string): Map<string, TDLDocument> {
  const documents = new Map<string, TDLDocument>();
  const sources: DocSource[] = [
    {
      baseDir: join(repoRoot, "docs", "analysis"),
      recursive: false,
      match: (p) => p.endsWith(".md"),
    },
    {
      baseDir: join(repoRoot, "docs", "requirements"),
      recursive: false,
      match: (p) => p.endsWith(".md"),
    },
    {
      baseDir: join(repoRoot, "docs", "adr"),
      recursive: false,
      match: (p) => p.endsWith(".md"),
    },
    {
      baseDir: join(repoRoot, "docs", "tasks"),
      recursive: true,
      match: (p) => p.endsWith("plan.md"),
    },
    {
      baseDir: join(repoRoot, "docs", "tasks"),
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
      ) {
        continue;
      }

      const doc = makeTDLDocument(filePath);
      documents.set(doc.docId, doc);
    }
  }

  return documents;
}

export function requirementDocsFrom(
  documents: Map<string, TDLDocument>,
): TDLDocument[] {
  return [...documents.values()].filter((doc) => doc.docType === "requirement");
}

export function taskDocsFrom(
  documents: Map<string, TDLDocument>,
): TDLDocument[] {
  return [...documents.values()].filter((doc) => doc.docType === "task");
}

export function documentsByLinkingRequirement(
  documents: Map<string, TDLDocument>,
  docType: DocumentType,
): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  for (const doc of documents.values()) {
    if (doc.docType !== docType) continue;
    const requirements = doc.links.requirements ?? [];
    for (const requirement of requirements) {
      const bucket = map.get(requirement) ?? new Set<string>();
      bucket.add(doc.docId);
      map.set(requirement, bucket);
    }
  }
  return map;
}

export function findImplementingTasks(
  documents: Map<string, TDLDocument>,
  reqId: string,
): string[] {
  const tasks: string[] = [];
  for (const doc of taskDocsFrom(documents)) {
    const linked = doc.links.requirements ?? [];
    if (linked.includes(reqId)) tasks.push(doc.docId);
  }
  return tasks;
}

export function findOrphanRequirements(
  documents: Map<string, TDLDocument>,
): string[] {
  const orphans: string[] = [];
  for (const doc of requirementDocsFrom(documents)) {
    if (findImplementingTasks(documents, doc.docId).length === 0) {
      orphans.push(doc.docId);
    }
  }
  return orphans;
}

export function findOrphanTasks(documents: Map<string, TDLDocument>): string[] {
  const orphans: string[] = [];
  for (const doc of taskDocsFrom(documents)) {
    if ((doc.links.requirements ?? []).length === 0) {
      orphans.push(doc.docId);
    }
  }
  return orphans;
}

export function calculateCoverage(
  documents: Map<string, TDLDocument>,
): CoverageReport {
  const requirements = requirementDocsFrom(documents);
  const tasks = taskDocsFrom(documents);
  const analyses = [...documents.values()].filter(
    (doc) => doc.docType === "analysis",
  );
  const adrs = [...documents.values()].filter((doc) => doc.docType === "adr");

  const requirementsWithTasks = requirements.filter(
    (req) => findImplementingTasks(documents, req.docId).length > 0,
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

function formatDocLink(doc: TDLDocument, outputDir: string): string {
  const relativePath = toPosixPath(relative(outputDir, doc.path));
  return `[${doc.docId}](${relativePath})`;
}

function formatPrimaryDoc(doc: TDLDocument, outputDir: string): string {
  const link = formatDocLink(doc, outputDir);
  if (!doc.title) return link;
  return `${link} - ${doc.title}`;
}

function formatLinkedDocs(
  documents: Map<string, TDLDocument>,
  ids: Iterable<string>,
  outputDir: string,
  options: { includeStatus?: boolean } = {},
): string {
  const includeStatus = options.includeStatus ?? false;
  const uniqueIds = [
    ...new Set([...ids].map((id) => id.trim()).filter(Boolean)),
  ];
  if (uniqueIds.length === 0) return "—";

  const parts = uniqueIds.map((id) => {
    const doc = documents.get(id);
    if (!doc) return id;
    const link = formatDocLink(doc, outputDir);
    if (includeStatus && doc.status !== "Unknown") {
      return `${link} (${doc.status})`;
    }
    return includeStatus ? `${link} (Unknown)` : link;
  });

  return parts.join("<br>");
}

export function renderTraceabilityMarkdown(
  documents: Map<string, TDLDocument>,
  outputPath: string,
): string {
  const outputDir = dirname(outputPath);
  const coverage = calculateCoverage(documents);
  const requirementDocuments = requirementDocsFrom(documents).sort((a, b) =>
    a.docId.localeCompare(b.docId),
  );
  const analysesByRequirement = documentsByLinkingRequirement(
    documents,
    "analysis",
  );
  const adrsByRequirement = documentsByLinkingRequirement(documents, "adr");

  const lines: string[] = [];
  lines.push("# Kopi Traceability Overview");
  lines.push("");
  lines.push(`Generated on ${new Date().toISOString()}`);
  lines.push("");

  lines.push("## Summary");
  lines.push("");
  lines.push("| Metric | Count |");
  lines.push("| --- | ---: |");
  lines.push(`| Analyses | ${coverage.total_analyses} |`);
  lines.push(`| Requirements | ${coverage.total_requirements} |`);
  lines.push(`| ADRs | ${coverage.total_adrs} |`);
  lines.push(`| Tasks | ${coverage.total_tasks} |`);
  lines.push(
    `| Requirements with tasks | ${coverage.requirements_with_tasks} (${coverage.coverage_percentage.toFixed(0)}%) |`,
  );
  lines.push("");

  lines.push("## Traceability Matrix");
  lines.push("");
  if (requirementDocuments.length === 0) {
    lines.push("No requirements found.");
  } else {
    lines.push("| Analyses | ADRs | Requirement | Status | Tasks |");
    lines.push("| --- | --- | --- | --- | --- |");
    for (const requirement of requirementDocuments) {
      const requirementLink = formatPrimaryDoc(requirement, outputDir);

      const taskIds = new Set<string>();
      for (const id of findImplementingTasks(documents, requirement.docId)) {
        taskIds.add(id);
      }
      for (const id of requirement.links.tasks ?? []) {
        taskIds.add(id);
      }

      const analysisIds = new Set<string>();
      for (const id of requirement.links.analyses ?? []) {
        analysisIds.add(id);
      }
      for (const id of analysesByRequirement.get(requirement.docId) ?? []) {
        analysisIds.add(id);
      }

      const adrIds = new Set<string>();
      for (const id of requirement.links.adrs ?? []) {
        adrIds.add(id);
      }
      for (const id of adrsByRequirement.get(requirement.docId) ?? []) {
        adrIds.add(id);
      }

      const analysesCell = formatLinkedDocs(documents, analysisIds, outputDir);
      const adrsCell = formatLinkedDocs(documents, adrIds, outputDir);
      const statusCell = requirement.status;
      const tasksCell = formatLinkedDocs(documents, taskIds, outputDir, {
        includeStatus: true,
      });

      lines.push(
        `| ${analysesCell} | ${adrsCell} | ${requirementLink} | ${statusCell} | ${tasksCell} |`,
      );
    }
  }

  lines.push("");
  lines.push("## Traceability Gaps");
  lines.push("");
  const orphanRequirements = findOrphanRequirements(documents).sort();
  const orphanTasks = findOrphanTasks(documents).sort();

  if (orphanRequirements.length === 0 && orphanTasks.length === 0) {
    lines.push("No gaps detected.");
  } else {
    for (const reqId of orphanRequirements) {
      const doc = documents.get(reqId);
      const status = doc?.status ?? "Unknown";
      lines.push(`- ${reqId}: No implementing task (Status: ${status})`);
    }
    for (const taskId of orphanTasks) {
      const doc = documents.get(taskId);
      const status = doc?.status ?? "Unknown";
      lines.push(`- ${taskId}: No linked requirements (Status: ${status})`);
    }
  }

  lines.push("");
  lines.push(
    "_This file is generated by `scripts/trace-status.ts`. Do not commit generated outputs to avoid merge conflicts._",
  );

  return lines.join("\n");
}

export function writeTraceabilityReport(
  documents: Map<string, TDLDocument>,
  outputPath: string,
): void {
  const content = renderTraceabilityMarkdown(documents, outputPath);
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, content, "utf8");
}

export function printStatus(
  documents: Map<string, TDLDocument>,
  gapsOnly: boolean,
): void {
  if (!gapsOnly) {
    console.log("=== Kopi TDL Status ===\n");
    const coverage = calculateCoverage(documents);
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

  const orphanRequirements = findOrphanRequirements(documents);
  const orphanTasks = findOrphanTasks(documents);

  if (orphanRequirements.length || orphanTasks.length) {
    console.log("Gaps:");
    for (const reqId of orphanRequirements.sort()) {
      const doc = documents.get(reqId);
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
    for (const doc of documents.values()) {
      let bucket = byType.get(doc.docType);
      if (!bucket) {
        bucket = [];
        byType.set(doc.docType, bucket);
      }
      bucket.push(doc);
    }

    for (const docType of ["analysis", "requirement", "adr", "task"] as const) {
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

export function checkIntegrity(documents: Map<string, TDLDocument>): boolean {
  const orphanRequirements = findOrphanRequirements(documents);
  const orphanTasks = findOrphanTasks(documents);

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

export function safeReadFile(path: string): string | null {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    console.error(
      `Warning: Failed to read ${path}: ${(error as Error).message}`,
    );
    return null;
  }
}

export function* walkFiles(
  rootDir: string,
  recursive: boolean,
): Generator<string> {
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

export function resolveLinkType(label: string): string | null {
  if (label.includes("requirement")) return "requirements";
  if (label.includes("analysis")) return "analyses";
  if (label.includes("adr")) return "adrs";
  if (label.includes("task")) return "tasks";
  if (label.includes("design")) return "design";
  if (label.includes("plan")) return "plan";
  return null;
}

export function extractIds(value: string): string[] {
  const matches = value.match(/[A-Z]+-[0-9a-z]{4,5}|[A-Z]+-\d+/g);
  return matches ? matches : [];
}

export function capitalize(value: string): string {
  if (!value) return value;
  return value.charAt(0).toUpperCase() + value.slice(1);
}

export function findRepoRoot(startDir: string): string | null {
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

export function parseArgs(argv: string[]): {
  gapsOnly: boolean;
  checkMode: boolean;
  writePath: string | null;
} {
  let gapsOnly = false;
  let checkMode = false;
  let writePath: string | null = null;
  for (const arg of argv) {
    if (arg === "--gaps") {
      gapsOnly = true;
    } else if (arg === "--check") {
      checkMode = true;
    } else if (arg === "--write") {
      writePath = "";
    } else if (arg.startsWith("--write=")) {
      writePath = arg.slice("--write=".length).trim();
      if (!writePath) {
        console.error("Error: --write= requires a path");
        process.exit(2);
      }
    } else {
      console.error(`Unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  return { gapsOnly, checkMode, writePath };
}

export function main(): number {
  const { gapsOnly, checkMode, writePath } = parseArgs(process.argv.slice(2));
  const repoRoot = findRepoRoot(process.cwd());
  if (!repoRoot) {
    console.error("Error: Could not find repository root");
    return 1;
  }

  const documents = loadDocuments(repoRoot);

  if (checkMode) {
    if (!checkIntegrity(documents)) {
      return 1;
    }
    if (writePath !== null) {
      const outputPath = resolveOutputPath(writePath, repoRoot);
      writeTraceabilityReport(documents, outputPath);
      console.log(
        `✓ Traceability check passed; report written to ${relative(
          repoRoot,
          outputPath,
        )}`,
      );
      return 0;
    }
    console.log("✓ Traceability check passed");
    return 0;
  }

  if (writePath !== null) {
    const outputPath = resolveOutputPath(writePath, repoRoot);
    writeTraceabilityReport(documents, outputPath);
    console.log(
      `Traceability report written to ${relative(repoRoot, outputPath)}`,
    );
  }

  printStatus(documents, gapsOnly);
  return 0;
}

export function resolveOutputPath(writePath: string, repoRoot: string): string {
  if (writePath === "") return join(repoRoot, "docs", "traceability.md");
  if (writePath.startsWith("/")) return writePath;
  return resolve(repoRoot, writePath);
}

export function toPosixPath(pathValue: string): string {
  return pathValue.split(sep).join("/");
}

if (import.meta.main) {
  process.exit(main());
}
