import { afterEach, beforeEach, describe, expect, it } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
	collectUsedIds,
	DEFAULT_ID_LENGTH,
	generateId,
	main,
	randomIntExclusive,
} from "./tdl-new-id";

const tempDirs: string[] = [];

function createTempDir(): string {
	const dir = mkdtempSync(join(tmpdir(), "tdl-new-id-test-"));
	tempDirs.push(dir);
	return dir;
}

afterEach(() => {
	while (tempDirs.length) {
		const dir = tempDirs.pop();
		if (dir === undefined) {
			continue;
		}
		rmSync(dir, { recursive: true, force: true });
	}
});

describe("randomIntExclusive", () => {
	it("returns values within range", () => {
		const max = 10;
		for (let i = 0; i < 100; i++) {
			const value = randomIntExclusive(max);
			expect(Number.isInteger(value)).toBe(true);
			expect(value).toBeGreaterThanOrEqual(0);
			expect(value).toBeLessThan(max);
		}
	});

	it("rejects invalid maximums", () => {
		expect(() => randomIntExclusive(0)).toThrow();
		expect(() => randomIntExclusive(257)).toThrow();
		expect(() => randomIntExclusive(1.5)).toThrow();
	});
});

describe("generateId", () => {
	it("creates base36 ids with the requested length", () => {
		const length = 8;
		const id = generateId(length);
		expect(id).toHaveLength(length);
		expect(id).toMatch(/^[0-9a-z]+$/);
	});
});

describe("collectUsedIds", () => {
	it("finds ids across nested directories", () => {
		const root = createTempDir();

		const analysisDir = join(root, "analysis");
		const tasksDir = join(root, "tasks", "T-1a2b3-task");
		mkdirSync(analysisDir, { recursive: true });
		mkdirSync(tasksDir, { recursive: true });

		writeFileSync(join(analysisDir, "AN-ab123-topic.md"), "");
		writeFileSync(join(tasksDir, "notes.txt"), "");

		const used = collectUsedIds(root);
		expect([...used].sort()).toEqual(["1a2b3", "ab123"]);
	});

	it("returns empty set when root does not exist", () => {
		const missing = join(createTempDir(), "missing");
		const used = collectUsedIds(missing);
		expect(used.size).toBe(0);
	});
});

describe("main", () => {
	const originalArgv = process.argv.slice();
	const originalEnv = { ...process.env };
	const originalLog = console.log;
	const originalError = console.error;
	let logCalls: unknown[][];
	let errorCalls: unknown[][];

	beforeEach(() => {
		logCalls = [];
		errorCalls = [];
		process.argv = ["bun", "tdl-new-id.ts"];
		console.log = (...args: unknown[]) => {
			logCalls.push(args);
		};
		console.error = (...args: unknown[]) => {
			errorCalls.push(args);
		};
	});

	afterEach(() => {
		process.argv = originalArgv.slice();

		for (const key of Object.keys(process.env)) {
			if (!(key in originalEnv)) {
				delete process.env[key];
			}
		}
		for (const [key, value] of Object.entries(originalEnv)) {
			process.env[key] = value;
		}

		console.log = originalLog;
		console.error = originalError;
	});

	it("prints a generated id with provided length", () => {
		const root = createTempDir();
		process.env.DOCS_DIR = root;
		process.env.ID_LEN = "4";

		const exitCode = main();

		expect(exitCode).toBe(0);
		expect(logCalls.length).toBe(1);
		const id = logCalls[0][0];
		expect(typeof id).toBe("string");
		expect(id as string).toHaveLength(4);
		expect(id as string).toMatch(/^[0-9a-z]+$/);
		expect(errorCalls.length).toBe(0);
	});

	it("falls back to default length when ID_LEN is invalid", () => {
		const root = createTempDir();
		process.env.DOCS_DIR = root;
		process.env.ID_LEN = "0";

		const exitCode = main();

		expect(exitCode).toBe(0);
		expect(logCalls.length).toBe(1);
		const id = logCalls[0][0];
		expect(typeof id).toBe("string");
		expect(id as string).toHaveLength(DEFAULT_ID_LENGTH);
		expect(errorCalls).toEqual([
			[`Warning: ID_LEN must be >= 1, using default ${DEFAULT_ID_LENGTH}`],
		]);
	});
});
