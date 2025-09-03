#!/usr/bin/env bun
/**
 * Generate a new 5-character random TDL document ID.
 * Uses base36 (0-9, a-z) for human readability.
 * Collision probability: ~1% at ~1,100 documents.
 * 
 * Environment Variables:
 *   ID_LEN: ID length (default: 5, min: 1)
 */

import { existsSync, readdirSync, statSync } from "fs";
import { join } from "path";

// Default ID length
const DEFAULT_ID_LENGTH = 5;

// Valid TDL prefixes
const VALID_PREFIXES = ['AN', 'FR', 'NFR', 'ADR', 'T'] as const;

/**
 * Generate a random base36 ID of specified length.
 */
function generateId(length: number = DEFAULT_ID_LENGTH): string {
    const chars = '0123456789abcdefghijklmnopqrstuvwxyz'; // 0-9, a-z
    let result = '';
    
    // Use crypto for secure random generation (equivalent to Python's secrets module)
    const randomBytes = new Uint8Array(length);
    crypto.getRandomValues(randomBytes);
    
    for (let i = 0; i < length; i++) {
        // Map each byte to a character in our charset
        result += chars[randomBytes[i] % chars.length];
    }
    
    return result;
}

/**
 * Recursively get all files and directories under a path.
 */
function* walkSync(dir: string): Generator<string> {
    if (!existsSync(dir)) return;
    
    const files = readdirSync(dir);
    
    for (const file of files) {
        const path = join(dir, file);
        yield path;
        
        if (statSync(path).isDirectory()) {
            yield* walkSync(path);
        }
    }
}

/**
 * Check if ID already exists in filenames/dirnames under docs/.
 */
function idExists(docId: string): boolean {
    const docsDir = 'docs';
    
    // If no docs directory yet, there can't be a collision
    if (!existsSync(docsDir)) {
        return false;
    }
    
    // Build all patterns to check (e.g., "AN-{id}-", "FR-{id}-", etc.)
    const patterns = VALID_PREFIXES.map(prefix => `${prefix}-${docId}-`);
    
    // Single pass through all files - O(files) instead of O(files * prefixes)
    for (const path of walkSync(docsDir)) {
        // Check if any pattern appears in the path
        if (patterns.some(pattern => path.includes(pattern))) {
            return true;
        }
    }
    
    return false;
}

/**
 * Main logic to generate unique ID.
 */
function main(): number {
    const maxAttempts = 10;
    
    // Robust input validation for ID_LEN
    let idLength = DEFAULT_ID_LENGTH;
    const idLenEnv = process.env.ID_LEN;
    
    if (idLenEnv !== undefined) {
        const parsed = parseInt(idLenEnv, 10);
        if (isNaN(parsed) || parsed < 1) {
            console.error(`Warning: ID_LEN must be >= 1, using default ${DEFAULT_ID_LENGTH}`);
            idLength = DEFAULT_ID_LENGTH;
        } else {
            idLength = parsed;
        }
    }
    
    for (let i = 0; i < maxAttempts; i++) {
        const newId = generateId(idLength);
        
        if (!idExists(newId)) {
            console.log(newId);
            return 0;
        }
        
        // ID collision detected, try again
        console.error(`ID collision detected for ${newId}, regenerating...`);
    }
    
    console.error(`Error: Could not generate unique ID after ${maxAttempts} attempts`);
    return 1;
}

// Run the main function and exit with its return code
process.exit(main());