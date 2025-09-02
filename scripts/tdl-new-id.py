#!/usr/bin/env python3
"""
Generate a new 5-character random TDL document ID.
Uses base36 (0-9, a-z) for human readability.
Collision probability: ~1% at ~1,100 documents.

Environment Variables:
  ID_LEN: ID length (default: 5, min: 1)
"""

import os
import secrets
import string
import sys
from pathlib import Path

# Default ID length
DEFAULT_ID_LENGTH = 5

# Valid TDL prefixes
VALID_PREFIXES = ('AN', 'FR', 'NFR', 'ADR', 'T')


def generate_id(length=DEFAULT_ID_LENGTH):
    """Generate a random base36 ID of specified length."""
    chars = string.digits + string.ascii_lowercase  # 0-9, a-z
    return ''.join(secrets.choice(chars) for _ in range(length))


def id_exists(doc_id):
    """Check if ID already exists in filenames/dirnames under docs/."""
    docs_dir = Path('docs')
    
    # If no docs directory yet, there can't be a collision
    if not docs_dir.exists():
        return False
    
    # Build all patterns to check (e.g., "AN-{id}-", "FR-{id}-", etc.)
    patterns = [f"{prefix}-{doc_id}-" for prefix in VALID_PREFIXES]
    
    # Single pass through all files - O(files) instead of O(files * prefixes)
    for path in docs_dir.rglob('*'):
        path_str = str(path)
        # Check if any pattern appears in the path
        if any(pattern in path_str for pattern in patterns):
            return True
    
    return False


def main():
    """Main logic to generate unique ID."""
    max_attempts = 10
    
    # Robust input validation for ID_LEN
    try:
        id_length = int(os.environ.get('ID_LEN', DEFAULT_ID_LENGTH))
        if id_length < 1:
            print(f"Warning: ID_LEN must be >= 1, using default {DEFAULT_ID_LENGTH}", file=sys.stderr)
            id_length = DEFAULT_ID_LENGTH
    except (ValueError, TypeError):
        print(f"Warning: Invalid ID_LEN, using default {DEFAULT_ID_LENGTH}", file=sys.stderr)
        id_length = DEFAULT_ID_LENGTH
    
    for _ in range(max_attempts):
        new_id = generate_id(id_length)
        
        if not id_exists(new_id):
            print(new_id)
            return 0
        
        # ID collision detected, try again
        print(f"ID collision detected for {new_id}, regenerating...", file=sys.stderr)
    
    print(f"Error: Could not generate unique ID after {max_attempts} attempts", file=sys.stderr)
    return 1


if __name__ == '__main__':
    sys.exit(main())