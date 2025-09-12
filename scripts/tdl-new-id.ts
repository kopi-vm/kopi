#!/usr/bin/env bun

/**
 * Generate a new random TDL document ID (base36, lowercase).
 * - Uniform randomness via rejection sampling (no modulo bias)
 * - Single-pass scan of docs to collect used IDs (no re-scan per attempt)
 * - Strict path matching on segment boundaries
 *
 * Environment Variables:
 *   ID_LEN   - ID length (default: 5, min: 1)
 *   DOCS_DIR - Root directory to scan (default: "docs")
 *
 * CLI (optional, minimal):
 *   tdl-new-id.ts [root]
 *   - If provided, positional [root] overrides DOCS_DIR.
 */

import { webcrypto as crypto } from "node:crypto";
import type { Dirent } from "fs";
import { existsSync, readdirSync } from "fs";
import { join } from "path";

// Default ID length
const DEFAULT_ID_LENGTH = 5;

// Valid TDL prefixes
const VALID_PREFIXES = ["AN", "FR", "NFR", "ADR", "T"] as const;

// Alphabet: 0-9, a-z
const ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyz" as const;

/**
 * Uniform random integer generator in [0, max).
 * Uses rejection sampling over a single random byte.
 */
function randomIntExclusive(max: number): number {
  if (!Number.isInteger(max) || max <= 0 || max > 256) {
    throw new Error(`randomIntExclusive: max must be in 1..256, got ${max}`);
  }
  const limit = Math.floor(256 / max) * max; // largest multiple of max < 256
  const buf = new Uint8Array(1);
  for (;;) {
    crypto.getRandomValues(buf);
    const val = buf[0];
    if (val < limit) return val % max;
  }
}

/**
 * Generate a random base36 ID of specified length using uniform selection.
 */
function generateId(length: number = DEFAULT_ID_LENGTH): string {
  let result = "";
  for (let i = 0; i < length; i++) {
    result += ALPHABET[randomIntExclusive(ALPHABET.length)];
  }
  return result;
}

/**
 * Recursively walk a directory tree using Dirent entries (no extra stat calls).
 */
function* walkPaths(rootDir: string): Generator<string> {
  if (!existsSync(rootDir)) return;
  const stack: string[] = [rootDir];
  while (stack.length) {
    const current = stack.pop() as string;
    let entries: Dirent[];
    try {
      entries = readdirSync(current, { withFileTypes: true });
    } catch {
      // Ignore directories we cannot read
      continue;
    }
    for (const dirent of entries) {
      const entryPath = join(current, dirent.name);
      yield entryPath;
      if (dirent.isDirectory()) stack.push(entryPath);
    }
  }
}

/**
 * Collect all used IDs from filenames/dirnames under the given root.
 * Matches segment-boundary patterns: (AN|FR|NFR|ADR|T)-<id>-
 */
function collectUsedIds(rootDir: string): Set<string> {
  const used = new Set<string>();
  if (!existsSync(rootDir)) return used;

  // e.g., .../requirements/FR-v7ql4-cache-locking.md or .../T-e7fa1-impl/
  const idPattern = new RegExp(
    String.raw`(?:^|[\\/])(?:${VALID_PREFIXES.join("|")})-([0-9a-z]+)-`,
    "g",
  );

  for (const p of walkPaths(rootDir)) {
    let m: RegExpExecArray | null;
    idPattern.lastIndex = 0; // reset for safety across loop
    while ((m = idPattern.exec(p)) !== null) {
      used.add(m[1]);
    }
  }
  return used;
}

/**
 * Main logic to generate unique ID (single scan, then in-memory checks).
 */
function main(): number {
  const maxAttempts = 10;

  // Root selection: positional arg > DOCS_DIR env > default "docs"
  const rootArg = process.argv[2];
  const docsRoot = rootArg?.trim() || process.env.DOCS_DIR || "docs";

  // Robust input validation for ID_LEN
  let idLength = DEFAULT_ID_LENGTH;
  const idLenEnv = process.env.ID_LEN;
  if (idLenEnv !== undefined) {
    const parsed = parseInt(idLenEnv, 10);
    if (Number.isNaN(parsed) || parsed < 1) {
      console.error(
        `Warning: ID_LEN must be >= 1, using default ${DEFAULT_ID_LENGTH}`,
      );
      idLength = DEFAULT_ID_LENGTH;
    } else {
      idLength = parsed;
    }
  }

  // Single pass: gather used IDs once
  const usedIds = collectUsedIds(docsRoot);

  for (let i = 0; i < maxAttempts; i++) {
    const newId = generateId(idLength);
    if (!usedIds.has(newId)) {
      console.log(newId);
      return 0;
    }
    // ID collision detected, try again
    console.error(`ID collision detected for ${newId}, regenerating...`);
  }

  console.error(
    `Error: Could not generate unique ID after ${maxAttempts} attempts`,
  );
  return 1;
}

// Run the main function and exit with its return code
process.exit(main());
