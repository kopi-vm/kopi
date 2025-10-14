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

function ensureSet(map: Map<string, Set<string>>, key: string): Set<string> {
  let bucket = map.get(key);
  if (!bucket) {
    bucket = new Set<string>();
    map.set(key, bucket);
  }
  return bucket;
}

type RequirementDependencyInfo = {
  directPrereqs: Set<string>;
  inferredPrereqs: Set<string>;
  directDependents: Set<string>;
  inferredDependents: Set<string>;
};

type InboundReferenceIndex = Map<string, Map<DocumentType, Set<string>>>;

type ReferenceRule = {
  sourceType: DocumentType;
  linkType: string;
  targetTypes: DocumentType[];
};

const HIERARCHY_REFERENCE_RULES: ReferenceRule[] = [
  {
    sourceType: "analysis",
    linkType: "requirements",
    targetTypes: ["requirement"],
  },
  { sourceType: "analysis", linkType: "tasks", targetTypes: ["task"] },
  { sourceType: "analysis", linkType: "adrs", targetTypes: ["adr"] },
  { sourceType: "adr", linkType: "requirements", targetTypes: ["requirement"] },
  { sourceType: "adr", linkType: "tasks", targetTypes: ["task"] },
  { sourceType: "requirement", linkType: "tasks", targetTypes: ["task"] },
];

function ensureDependencyInfo(
  map: Map<string, RequirementDependencyInfo>,
  docId: string,
): RequirementDependencyInfo {
  let info = map.get(docId);
  if (!info) {
    info = {
      directPrereqs: new Set<string>(),
      inferredPrereqs: new Set<string>(),
      directDependents: new Set<string>(),
      inferredDependents: new Set<string>(),
    };
    map.set(docId, info);
  }
  return info;
}

function ensureInboundBucket(
  index: InboundReferenceIndex,
  targetId: string,
): Map<DocumentType, Set<string>> {
  let bucket = index.get(targetId);
  if (!bucket) {
    bucket = new Map<DocumentType, Set<string>>();
    index.set(targetId, bucket);
  }
  return bucket;
}

function ensureInboundSet(
  bucket: Map<DocumentType, Set<string>>,
  sourceType: DocumentType,
): Set<string> {
  let references = bucket.get(sourceType);
  if (!references) {
    references = new Set<string>();
    bucket.set(sourceType, references);
  }
  return references;
}

export function buildInboundReferenceIndex(
  documents: Map<string, TDLDocument>,
): InboundReferenceIndex {
  const inbound: InboundReferenceIndex = new Map();

  for (const doc of documents.values()) {
    for (const rule of HIERARCHY_REFERENCE_RULES) {
      if (doc.docType !== rule.sourceType) continue;
      const targets = doc.links[rule.linkType];
      if (!targets) continue;
      for (const targetId of targets) {
        const targetDoc = documents.get(targetId);
        if (!targetDoc) continue;
        if (!rule.targetTypes.includes(targetDoc.docType)) continue;
        const bucket = ensureInboundBucket(inbound, targetDoc.docId);
        ensureInboundSet(bucket, doc.docType).add(doc.docId);
      }
    }
  }

  return inbound;
}

export type DocumentSourceInfo = {
  readonly path: string;
  readonly filename: string;
  readonly headingId: string | null;
};

export type TDLDocument = {
  readonly path: string;
  readonly filename: string;
  readonly docId: string;
  readonly docType: DocumentType;
  readonly links: LinkMap;
  readonly status: string;
  readonly metadataType: string;
  readonly title: string;
  readonly sources: readonly DocumentSourceInfo[];
};

export function makeTDLDocument(filePath: string): TDLDocument {
  const filename = filePath.split(/[/\\]/).pop() ?? filePath;
  const content = safeReadFile(filePath);
  const docId = extractDocumentId(filename, filePath, content);
  const headingId = extractFirstLineId(content);
  return {
    path: filePath,
    filename,
    docId,
    docType: inferDocumentType(filename, filePath),
    links: parseDocumentLinks(content),
    status: extractDocumentStatus(content),
    metadataType: extractDocumentMetadataType(content),
    title: extractDocumentTitle(content),
    sources: [
      {
        path: filePath,
        filename,
        headingId,
      },
    ],
  };
}

function mergeTDLDocuments(
  existing: TDLDocument,
  incoming: TDLDocument,
): TDLDocument {
  const mergedLinks = mergeLinkMaps(existing.links, incoming.links);
  const preferredForPath = preferDocument(
    existing,
    incoming,
    documentSourcePriority,
  );
  const preferredForStatus = preferDocument(existing, incoming, statusPriority);
  const preferredForTitle = preferDocument(
    existing,
    incoming,
    documentSourcePriority,
  );

  const mergedDocType =
    existing.docType === "unknown" && incoming.docType !== "unknown"
      ? incoming.docType
      : existing.docType;

  const mergedMetadataType = selectMetadataType(existing, incoming);
  const mergedSources = mergeDocumentSources(
    existing.sources,
    incoming.sources,
  );

  return {
    path: preferredForPath.path,
    filename: preferredForPath.filename,
    docId: existing.docId,
    docType: mergedDocType,
    links: mergedLinks,
    status: preferredForStatus.status,
    metadataType: mergedMetadataType,
    title: preferredForTitle.title,
    sources: mergedSources,
  };
}

function mergeLinkMaps(first: LinkMap, second: LinkMap): LinkMap {
  const merged: LinkMap = {};
  const keys = new Set([...Object.keys(first), ...Object.keys(second)]);
  for (const key of keys) {
    const combined = [...(first[key] ?? []), ...(second[key] ?? [])]
      .map((value) => value.trim())
      .filter(Boolean);
    const unique = [...new Set(combined)].sort((a, b) => a.localeCompare(b));
    if (unique.length > 0) {
      merged[key] = unique;
    }
  }
  return merged;
}

function mergeDocumentSources(
  first: readonly DocumentSourceInfo[],
  second: readonly DocumentSourceInfo[],
): DocumentSourceInfo[] {
  const byPath = new Map<string, DocumentSourceInfo>();

  const record = (source: DocumentSourceInfo) => {
    const existing = byPath.get(source.path);
    if (!existing) {
      byPath.set(source.path, source);
      return;
    }
    if (!existing.headingId && source.headingId) {
      byPath.set(source.path, source);
    }
  };

  for (const source of first) {
    record(source);
  }
  for (const source of second) {
    record(source);
  }

  return [...byPath.values()];
}

type PrioritySelector = (doc: TDLDocument) => number;

function preferDocument(
  existing: TDLDocument,
  incoming: TDLDocument,
  selector: PrioritySelector,
): TDLDocument {
  const existingScore = selector(existing);
  const incomingScore = selector(incoming);
  return incomingScore < existingScore ? incoming : existing;
}

function documentSourcePriority(doc: TDLDocument): number {
  const name = doc.filename.toLowerCase();
  if (name === "readme.md") return 0;
  if (name === "plan.md") return 1;
  if (name === "design.md") return 2;
  return 5;
}

function statusPriority(doc: TDLDocument): number {
  const base = isMeaningfulStatus(doc.status) ? 0 : 20;
  return base + documentSourcePriority(doc);
}

function isMeaningfulStatus(status: string): boolean {
  if (!status) return false;
  if (status === "Unknown") return false;
  return true;
}

function selectMetadataType(
  existing: TDLDocument,
  incoming: TDLDocument,
): string {
  const existingType = sanitizeMetadataType(existing.metadataType);
  const incomingType = sanitizeMetadataType(incoming.metadataType);

  const existingScore = metadataPriority(existing, existingType);
  const incomingScore = metadataPriority(incoming, incomingType);

  if (incomingScore < existingScore) return incomingType;
  return existingType;
}

function sanitizeMetadataType(value: string | undefined): string {
  if (!value) return "";
  if (isTemplatePlaceholder(value)) return "";
  return value.trim();
}

function metadataPriority(doc: TDLDocument, metadataType: string): number {
  const base = metadataType ? 0 : 20;
  return base + documentSourcePriority(doc);
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
  if (statusMatch) {
    const raw = statusMatch[1].split("<!--")[0].trim();
    if (isTemplatePlaceholder(raw)) return "Unknown";
    return raw;
  }

  return "Unknown";
}

export function extractDocumentTitle(content: string | null): string {
  if (content === null) return "";
  const match = content.match(/^#\s+(.+)$/m);
  return match ? match[1].trim() : "";
}

export function extractFirstLineId(content: string | null): string | null {
  if (content === null) return null;
  const [firstLineRaw] = content.split(/\r?\n/, 1);
  if (firstLineRaw === undefined) return null;
  const firstLine = firstLineRaw.replace(/^\uFEFF/, "").trim();
  if (!firstLine) return null;

  const headingMatch = firstLine.match(/^#\s+([A-Z]+-[0-9A-Za-z]+)/);
  if (headingMatch) return headingMatch[1];

  const idMatch = firstLine.match(/^([A-Z]+-[0-9A-Za-z]+)/);
  if (idMatch) return idMatch[1];
  return null;
}

export function extractDocumentMetadataType(content: string | null): string {
  if (content === null) return "";

  const typeMatch = content.match(/^\s*-\s*Type:\s*(.+)$/m);
  if (!typeMatch) return "";

  const raw = typeMatch[1].split("<!--")[0].trim();
  if (isTemplatePlaceholder(raw)) return "";
  return raw;
}

function isTemplatePlaceholder(value: string): boolean {
  if (!value) return true;
  if (value.includes("|")) return true;
  if (/^`.*`$/.test(value)) return true;
  if (/^\[.*\]$/.test(value)) return true;
  return false;
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
    {
      baseDir: join(repoRoot, "docs", "tasks"),
      recursive: true,
      match: (p) => p.endsWith("README.md"),
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
      const existing = documents.get(doc.docId);
      if (existing) {
        documents.set(doc.docId, mergeTDLDocuments(existing, doc));
      } else {
        documents.set(doc.docId, doc);
      }
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

function buildRequirementDependencyInfo(documents: Map<string, TDLDocument>): {
  infoByRequirement: Map<string, RequirementDependencyInfo>;
  missingPrereqs: Map<string, Set<string>>;
  missingDependents: Map<string, Set<string>>;
} {
  const infoByRequirement = new Map<string, RequirementDependencyInfo>();

  for (const requirement of requirementDocsFrom(documents)) {
    const info = ensureDependencyInfo(infoByRequirement, requirement.docId);

    const addPrereq = (rawId: string) => {
      const id = rawId.trim();
      if (!id || id === requirement.docId) return;
      info.directPrereqs.add(id);
    };

    const addDependent = (rawId: string) => {
      const id = rawId.trim();
      if (!id || id === requirement.docId) return;
      info.directDependents.add(id);
    };

    for (const id of requirement.links.depends_on ?? []) {
      addPrereq(id);
    }
    for (const id of requirement.links.blocked_by ?? []) {
      addPrereq(id);
    }
    for (const id of requirement.links.requirements ?? []) {
      addPrereq(id);
    }
    for (const id of requirement.links.blocks ?? []) {
      addDependent(id);
    }
  }

  // Derive reciprocal relationships.
  for (const [docId, info] of infoByRequirement) {
    for (const prereq of info.directPrereqs) {
      const prereqInfo = ensureDependencyInfo(infoByRequirement, prereq);
      if (prereq !== docId) {
        prereqInfo.inferredDependents.add(docId);
      }
    }
    for (const dependent of info.directDependents) {
      const dependentInfo = ensureDependencyInfo(infoByRequirement, dependent);
      if (dependent !== docId) {
        dependentInfo.inferredPrereqs.add(docId);
      }
    }
  }

  const missingPrereqs = new Map<string, Set<string>>();
  const missingDependents = new Map<string, Set<string>>();

  for (const [docId, info] of infoByRequirement) {
    for (const inferred of info.inferredPrereqs) {
      if (!info.directPrereqs.has(inferred)) {
        ensureSet(missingPrereqs, docId).add(inferred);
      }
    }
    for (const inferred of info.inferredDependents) {
      if (!info.directDependents.has(inferred)) {
        ensureSet(missingDependents, docId).add(inferred);
      }
    }
  }

  return { infoByRequirement, missingPrereqs, missingDependents };
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
  const tasks = new Set<string>();
  const requirementDoc = documents.get(reqId);
  if (requirementDoc && requirementDoc.docType === "requirement") {
    for (const taskId of requirementDoc.links.tasks ?? []) {
      const taskDoc = documents.get(taskId);
      if (taskDoc && taskDoc.docType === "task") {
        tasks.add(taskDoc.docId);
      }
    }
  }

  for (const doc of documents.values()) {
    if (doc.docType !== "adr") continue;
    const linkedRequirements = doc.links.requirements ?? [];
    if (!linkedRequirements.includes(reqId)) continue;
    for (const taskId of doc.links.tasks ?? []) {
      const taskDoc = documents.get(taskId);
      if (taskDoc && taskDoc.docType === "task") {
        tasks.add(taskDoc.docId);
      }
    }
  }

  return [...tasks].sort((a, b) => a.localeCompare(b));
}

export function findOrphanRequirements(
  documents: Map<string, TDLDocument>,
  inboundReferences?: InboundReferenceIndex,
): string[] {
  const inbound = inboundReferences ?? buildInboundReferenceIndex(documents);
  const orphans: string[] = [];
  for (const doc of requirementDocsFrom(documents)) {
    const references = inbound.get(doc.docId);
    const hasAnalysis = (references?.get("analysis")?.size ?? 0) > 0;
    const hasAdr = (references?.get("adr")?.size ?? 0) > 0;
    if (!hasAnalysis && !hasAdr) {
      orphans.push(doc.docId);
    }
  }
  return orphans.sort((a, b) => a.localeCompare(b));
}

export function findOrphanAdrs(
  documents: Map<string, TDLDocument>,
  inboundReferences?: InboundReferenceIndex,
): string[] {
  const inbound = inboundReferences ?? buildInboundReferenceIndex(documents);
  const orphans: string[] = [];
  for (const doc of documents.values()) {
    if (doc.docType !== "adr") continue;
    const references = inbound.get(doc.docId);
    const hasAnalysis = (references?.get("analysis")?.size ?? 0) > 0;
    if (!hasAnalysis) {
      orphans.push(doc.docId);
    }
  }
  return orphans.sort((a, b) => a.localeCompare(b));
}

export function findOrphanTasks(
  documents: Map<string, TDLDocument>,
  inboundReferences?: InboundReferenceIndex,
): string[] {
  const inbound = inboundReferences ?? buildInboundReferenceIndex(documents);
  const orphans: string[] = [];
  for (const doc of taskDocsFrom(documents)) {
    const references = inbound.get(doc.docId);
    const hasAnalysis = (references?.get("analysis")?.size ?? 0) > 0;
    const hasRequirement = (references?.get("requirement")?.size ?? 0) > 0;
    const hasAdr = (references?.get("adr")?.size ?? 0) > 0;
    if (!hasAnalysis && !hasRequirement && !hasAdr) {
      orphans.push(doc.docId);
    }
  }
  return orphans.sort((a, b) => a.localeCompare(b));
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

function formatSingleLink(
  documents: Map<string, TDLDocument>,
  id: string,
  outputDir: string,
  options: { includeStatus?: boolean } = {},
): string {
  const includeStatus = options.includeStatus ?? false;
  const doc = documents.get(id);
  if (!doc) return id;
  const link = formatDocLink(doc, outputDir);
  if (!includeStatus) return link;
  const status = doc.status !== "Unknown" ? doc.status : "Unknown";
  return `${link} (${status})`;
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

  const parts = uniqueIds.map((id) =>
    formatSingleLink(documents, id, outputDir, { includeStatus }),
  );

  return parts.join("<br>");
}

function formatDependencyCell(
  documents: Map<string, TDLDocument>,
  direct: Iterable<string>,
  inferred: Iterable<string>,
  outputDir: string,
): string {
  const originById = new Map<string, "direct" | "inferred">();

  const record = (rawId: string, origin: "direct" | "inferred") => {
    const id = rawId.trim();
    if (!id) return;
    if (originById.has(id)) {
      if (originById.get(id) === "direct") return;
    }
    originById.set(id, origin);
  };

  for (const id of direct) {
    record(id, "direct");
  }
  for (const id of inferred) {
    record(id, "inferred");
  }

  if (originById.size === 0) return "—";

  const parts = [...originById.keys()]
    .sort((a, b) => a.localeCompare(b))
    .map((id) => {
      const origin = originById.get(id);
      const link = formatSingleLink(documents, id, outputDir);
      if (origin === "inferred") {
        return `${link} (inferred)`;
      }
      return link;
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
  const inboundReferences = buildInboundReferenceIndex(documents);
  const analysesByRequirement = documentsByLinkingRequirement(
    documents,
    "analysis",
  );
  const adrsByRequirement = documentsByLinkingRequirement(documents, "adr");
  const { infoByRequirement, missingPrereqs, missingDependents } =
    buildRequirementDependencyInfo(documents);

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
  lines.push("### Requirement Dependencies");
  lines.push("");

  if (requirementDocuments.length === 0) {
    lines.push("No requirement dependencies found.");
  } else {
    lines.push("| Requirement | Depends On | Blocks | Blocked By |");
    lines.push("| --- | --- | --- | --- |");
    for (const requirement of requirementDocuments) {
      const requirementLink = formatPrimaryDoc(requirement, outputDir);
      const info = infoByRequirement.get(requirement.docId) ?? {
        directPrereqs: new Set<string>(),
        inferredPrereqs: new Set<string>(),
        directDependents: new Set<string>(),
        inferredDependents: new Set<string>(),
      };
      const dependsCell = formatDependencyCell(
        documents,
        info.directPrereqs,
        info.inferredPrereqs,
        outputDir,
      );
      const blocksCell = formatDependencyCell(
        documents,
        info.directDependents,
        info.inferredDependents,
        outputDir,
      );
      const blockedByCell = formatDependencyCell(
        documents,
        info.directPrereqs,
        info.inferredPrereqs,
        outputDir,
      );

      lines.push(
        `| ${requirementLink} | ${dependsCell} | ${blocksCell} | ${blockedByCell} |`,
      );
    }
  }

  lines.push("");
  lines.push("### Dependency Consistency");
  lines.push("");

  const hasMissingPrereqs = missingPrereqs.size > 0;
  const hasMissingDependents = missingDependents.size > 0;

  if (!hasMissingPrereqs && !hasMissingDependents) {
    lines.push(
      "All prerequisites and dependents are documented on both sides.",
    );
  } else {
    const missingPrereqEntries = [...missingPrereqs.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    );
    for (const [reqId, missing] of missingPrereqEntries) {
      const doc = documents.get(reqId);
      const requirementLink = doc ? formatPrimaryDoc(doc, outputDir) : reqId;
      const missingCell = formatDependencyCell(
        documents,
        new Set<string>(),
        missing,
        outputDir,
      );
      lines.push(
        `- ${requirementLink}: add Prerequisite Requirements entry for ${missingCell}`,
      );
    }

    const missingDependentEntries = [...missingDependents.entries()].sort(
      (a, b) => a[0].localeCompare(b[0]),
    );
    for (const [reqId, missing] of missingDependentEntries) {
      const doc = documents.get(reqId);
      const requirementLink = doc ? formatPrimaryDoc(doc, outputDir) : reqId;
      const missingCell = formatDependencyCell(
        documents,
        new Set<string>(),
        missing,
        outputDir,
      );
      lines.push(
        `- ${requirementLink}: add Dependent Requirements entry for ${missingCell}`,
      );
    }
  }

  lines.push("");
  lines.push("## Traceability Gaps");
  lines.push("");
  const orphanRequirements = findOrphanRequirements(
    documents,
    inboundReferences,
  );
  const orphanAdrs = findOrphanAdrs(documents, inboundReferences);
  const orphanTasks = findOrphanTasks(documents, inboundReferences);

  if (
    orphanRequirements.length === 0 &&
    orphanAdrs.length === 0 &&
    orphanTasks.length === 0
  ) {
    lines.push("No gaps detected.");
  } else {
    for (const reqId of orphanRequirements) {
      const doc = documents.get(reqId);
      const status = doc?.status ?? "Unknown";
      lines.push(
        `- ${reqId}: No upstream analysis or ADR references (Status: ${status})`,
      );
    }
    for (const adrId of orphanAdrs) {
      const doc = documents.get(adrId);
      const status = doc?.status ?? "Unknown";
      lines.push(
        `- ${adrId}: No upstream analysis references (Status: ${status})`,
      );
    }
    for (const taskId of orphanTasks) {
      const doc = documents.get(taskId);
      const status = doc?.status ?? "Unknown";
      lines.push(
        `- ${taskId}: No upstream analysis, requirement, or ADR references (Status: ${status})`,
      );
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
  const { missingPrereqs, missingDependents } =
    buildRequirementDependencyInfo(documents);
  const inboundReferences = buildInboundReferenceIndex(documents);
  const headingMismatches = findHeadingMismatches(documents);
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

  const orphanRequirements = findOrphanRequirements(
    documents,
    inboundReferences,
  );
  const orphanAdrs = findOrphanAdrs(documents, inboundReferences);
  const orphanTasks = findOrphanTasks(documents, inboundReferences);

  if (orphanRequirements.length || orphanAdrs.length || orphanTasks.length) {
    console.log("Gaps:");
    for (const reqId of orphanRequirements) {
      const doc = documents.get(reqId);
      const status = doc?.status ?? "Unknown";
      console.log(
        `  ⚠ ${reqId}: No upstream analysis or ADR references (Status: ${status})`,
      );
    }
    for (const adrId of orphanAdrs) {
      const doc = documents.get(adrId);
      const status = doc?.status ?? "Unknown";
      console.log(
        `  ⚠ ${adrId}: No upstream analysis references (Status: ${status})`,
      );
    }
    for (const taskId of orphanTasks) {
      const doc = documents.get(taskId);
      const status = doc?.status ?? "Unknown";
      console.log(
        `  ⚠ ${taskId}: No upstream analysis, requirement, or ADR references (Status: ${status})`,
      );
    }
    console.log();
  } else if (!gapsOnly) {
    console.log("✓ No gaps detected\n");
  }

  if (missingPrereqs.size || missingDependents.size) {
    console.log("Dependency consistency issues:");
    const prereqEntries = [...missingPrereqs.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    );
    for (const [reqId, missing] of prereqEntries) {
      const missingList = [...missing].sort((a, b) => a.localeCompare(b));
      console.log(
        `  ⚠ ${reqId}: Missing prerequisite link(s) for ${missingList.join(", ")}`,
      );
    }
    const dependentEntries = [...missingDependents.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    );
    for (const [reqId, missing] of dependentEntries) {
      const missingList = [...missing].sort((a, b) => a.localeCompare(b));
      console.log(
        `  ⚠ ${reqId}: Missing dependent link(s) for ${missingList.join(", ")}`,
      );
    }
    console.log();
  } else if (!gapsOnly) {
    console.log("Dependency links consistent\n");
  }

  if (headingMismatches.length) {
    console.log("Document ID heading mismatches detected:");
    for (const mismatch of headingMismatches) {
      const location = relative(process.cwd(), mismatch.path);
      const displayPath = location.startsWith("..")
        ? mismatch.path
        : location || mismatch.path;
      const actual = mismatch.actualId ?? "<missing>";
      console.log(
        `  ⚠ ${displayPath}: expected ${mismatch.expectedId} on first line, found ${actual}`,
      );
    }
    console.log();
  } else if (!gapsOnly) {
    console.log("Document ID headings consistent\n");
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
      console.log(`\n  ${docTypeDisplayName(docType)}:`);
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

export type HeadingMismatch = {
  readonly path: string;
  readonly expectedId: string;
  readonly actualId: string | null;
};

export function findHeadingMismatches(
  documents: Map<string, TDLDocument>,
): HeadingMismatch[] {
  const idPattern = /^[A-Z]+-[0-9A-Za-z]+$/;
  const mismatches: HeadingMismatch[] = [];
  for (const doc of documents.values()) {
    if (!idPattern.test(doc.docId)) continue;
    if (doc.docType === "unknown") continue;
    for (const source of doc.sources) {
      if (source.headingId === doc.docId) continue;
      mismatches.push({
        path: source.path,
        expectedId: doc.docId,
        actualId: source.headingId,
      });
    }
  }
  return mismatches.sort((a, b) => a.path.localeCompare(b.path));
}

export function checkIntegrity(documents: Map<string, TDLDocument>): boolean {
  const inboundReferences = buildInboundReferenceIndex(documents);
  const orphanRequirements = findOrphanRequirements(
    documents,
    inboundReferences,
  );
  const orphanAdrs = findOrphanAdrs(documents, inboundReferences);
  const orphanTasks = findOrphanTasks(documents, inboundReferences);
  const { missingPrereqs, missingDependents } =
    buildRequirementDependencyInfo(documents);

  let ok = true;

  if (orphanRequirements.length || orphanAdrs.length || orphanTasks.length) {
    console.error("Traceability gaps detected:");
    for (const reqId of orphanRequirements) {
      console.error(`  - ${reqId}: No upstream analysis or ADR references`);
    }
    for (const adrId of orphanAdrs) {
      console.error(`  - ${adrId}: No upstream analysis references`);
    }
    for (const taskId of orphanTasks) {
      console.error(
        `  - ${taskId}: No upstream analysis, requirement, or ADR references`,
      );
    }
    ok = false;
  }

  if (missingPrereqs.size || missingDependents.size) {
    console.error("Dependency consistency issues detected:");
    const prereqEntries = [...missingPrereqs.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    );
    for (const [reqId, missing] of prereqEntries) {
      const missingList = [...missing].sort((a, b) => a.localeCompare(b));
      console.error(
        `  - ${reqId}: Missing prerequisite link(s) for ${missingList.join(", ")}`,
      );
    }
    const dependentEntries = [...missingDependents.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    );
    for (const [reqId, missing] of dependentEntries) {
      const missingList = [...missing].sort((a, b) => a.localeCompare(b));
      console.error(
        `  - ${reqId}: Missing dependent link(s) for ${missingList.join(", ")}`,
      );
    }
    ok = false;
  }

  const headingMismatches = findHeadingMismatches(documents);
  if (headingMismatches.length) {
    console.error("Document ID heading mismatches detected:");
    for (const mismatch of headingMismatches) {
      const location = relative(process.cwd(), mismatch.path);
      const displayPath = location.startsWith("..")
        ? mismatch.path
        : location || mismatch.path;
      const actual = mismatch.actualId ?? "<missing>";
      console.error(
        `  - ${displayPath}: expected ${mismatch.expectedId} on first line, found ${actual}`,
      );
    }
    ok = false;
  }

  return ok;
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
  const normalized = label.toLowerCase();
  if (normalized.includes("prerequisite")) return "depends_on";
  if (normalized.includes("dependent")) return "blocks";
  if (normalized.includes("depend")) return "depends_on";
  if (normalized.includes("analys")) return "analyses";
  if (normalized.includes("adr")) return "adrs";
  if (normalized.includes("task")) return "tasks";
  if (normalized.includes("requirement")) return "requirements";
  return null;
}

export function extractIds(value: string): string[] {
  const matches = value.match(/\b[A-Z]+-[0-9a-z]+\b/g);
  return matches ? matches : [];
}

export function capitalize(value: string): string {
  if (!value) return value;
  return value.charAt(0).toUpperCase() + value.slice(1);
}

const DOC_TYPE_DISPLAY_NAMES: Record<DocumentType, string> = {
  analysis: "Analyses",
  requirement: "Requirements",
  adr: "ADRs",
  task: "Tasks",
  unknown: "Unknown Documents",
};

function docTypeDisplayName(docType: DocumentType): string {
  return DOC_TYPE_DISPLAY_NAMES[docType] ?? capitalize(docType);
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
