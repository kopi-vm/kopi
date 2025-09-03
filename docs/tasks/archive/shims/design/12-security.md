# Security Considerations

## Path Validation

The shim system must ensure that all resolved paths remain within expected directories. When validating tool paths, the system should verify that the path is located within the kopi directory structure (typically ~/.kopi). Additionally, it should confirm that the file exists and has appropriate executable permissions. On Unix systems, this includes checking that the file has execute permission bits set.

## Symlink Protection

On Unix systems, symlinks require special attention to prevent security vulnerabilities. The system should validate that symlink targets remain within the shim directory and don't point to locations outside the expected boundaries. This involves resolving relative paths correctly and using canonical paths to eliminate any directory traversal attempts through ".." or "." components.

## Input Sanitization

Version strings read from configuration files must be properly sanitized to prevent various attacks. The system should:

- Limit input length to reasonable bounds (e.g., 100 characters)
- Validate that only expected characters are present (alphanumeric, dots, hyphens, underscores, and @ symbols)
- Reject any input containing path traversal sequences like "..", "/" or "\"
- Trim whitespace to ensure consistent processing

## Permission Checks

Before executing any tool, the system must verify appropriate file permissions:

- Confirm the file is executable by the current user
- Issue warnings if files are world-writable, as this could indicate a security risk
- Verify file ownership matches the current user where possible
- Check for any permission anomalies that could indicate tampering

## Download Verification

When auto-installing JDKs, the system must ensure download integrity through cryptographic verification. This involves:

- Computing SHA-256 hashes of downloaded files
- Comparing computed hashes against expected values from trusted sources
- Immediately removing any files that fail verification
- Providing clear error messages about verification failures

## Environment Variable Injection

Environment variables passed to child processes require careful handling:

- Limit the length of environment variable values to prevent overflow attacks
- Validate that values don't contain null bytes or other problematic characters
- Sanitize any user-controlled values before passing them to subprocesses
- Consider allowlisting specific environment variables rather than passing all

## Security Principles

The shim system adheres to these fundamental security principles:

1. **Principle of Least Privilege**: Shims operate exclusively with user permissions and never require elevated privileges
2. **Input Validation**: All user input undergoes thorough validation and sanitization before use
3. **Path Validation**: File paths are strictly validated to prevent directory traversal attacks
4. **No Elevated Privileges**: The system never requires or uses sudo/administrator rights
5. **Secure Downloads**: All JDK downloads use HTTPS and undergo checksum verification
6. **Limited Tool Exposure**: Only a curated list of user-facing tools (java, javac, etc.) are exposed through shims

## Common Attack Vectors and Mitigations

The system protects against these common attack vectors:

| Attack Vector         | Mitigation Strategy                                                                                         |
| --------------------- | ----------------------------------------------------------------------------------------------------------- |
| Path Traversal        | Validate all file paths and reject sequences containing ".." or absolute paths outside expected directories |
| Symlink Attacks       | Verify symlink targets remain within kopi directories and resolve to expected locations                     |
| Command Injection     | Never construct shell commands from user input; use direct process execution instead                        |
| Download MITM         | Use HTTPS exclusively for downloads and verify checksums against trusted sources                            |
| Privilege Escalation  | Design system to work entirely with user permissions; never require elevated privileges                     |
| Environment Injection | Sanitize and validate all environment variables before passing to child processes                           |

## Next: [Migration and Compatibility](./13-migration-compatibility.md)
