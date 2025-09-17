# Creating and Maintaining Curated Tool Lists

The curated tool lists are created through several methods:

## 1. Official Documentation Analysis

We analyze official vendor documentation to identify all tools provided by each JDK distribution. This involves:

- Scanning documentation sites for each major JDK vendor (Temurin, GraalVM, Corretto, etc.)
- Extracting tool names and descriptions from reference manuals
- Building a comprehensive database of available tools per vendor
- Maintaining URLs to official documentation for each vendor

This automated analysis helps us stay current with new tools as vendors update their distributions.

## 2. Empirical Testing

We perform empirical testing by downloading actual JDK releases and scanning their contents:

- Download multiple versions of each distribution (e.g., Temurin 17.0.9, 21.0.1, GraalVM 17.0.9)
- Extract the JDK archives and scan the bin directory
- Identify all executable files that are user-facing tools
- Filter out non-user tools like:
  - Hidden files (starting with .)
  - System libraries (.dll, .so, .dylib files)
  - Deprecated tools (rmid, tnameserv, pack200, unpack200)
- Verify executability on each platform (checking file permissions on Unix, .exe extension on Windows)
- Compare tool lists across versions to identify additions and removals

## 3. Community Feedback Integration

We maintain version-controlled tool lists that incorporate community feedback:

**Standard Tools** (available in all JDK distributions):

- Core development: java, javac, javap, javadoc
- Packaging and signing: jar, jarsigner
- Interactive tools: jshell
- Build tools: jlink, jmod, jdeps
- Debugging: jconsole, jdb
- Security: keytool
- Monitoring: jfr, jcmd, jinfo, jmap, jps, jstack, jstat, jstatd
- Other utilities: rmiregistry, serialver

**Vendor-Specific Tools**:

- GraalVM adds: gu (GraalVM Updater), native-image, polyglot, lli, js
  - Note: js tool was removed in GraalVM 23.0.0
- Corretto 11 includes: jmc (Java Mission Control)
  - Note: jmc was removed in Corretto 17+

Each tool can have metadata including:

- Description of its purpose
- Whether it's vendor-specific
- Version constraints (added/removed in specific versions)
- Dependencies (e.g., native-image requires the native-image component)

## 4. Automated Validation

We use continuous integration to validate our tool lists:

- Test that deprecated tools are never included
- Verify essential tools (java, javac) are always present
- Check version-specific rules are correctly applied
- Validate tool availability across different distributions
- Ensure tool lists remain accurate as new JDK versions are released

## 5. Update Process

We maintain tool lists through a systematic update process:

1. **Quarterly Reviews**: Check vendor release notes for tool changes
2. **Automated Scanning**: CI job that downloads latest JDKs and compares tools
3. **Issue Tracking**: Users can report missing tools via GitHub issues
4. **Version-Aware**: Track when tools are added/removed in specific versions

### Version-Aware Tool Resolution

The system intelligently adjusts tool lists based on the JDK version:

- Start with the standard tool set that all JDKs provide
- For GraalVM:
  - Add vendor tools: gu, native-image, polyglot
  - Include js tool only for versions before 23.0.0
- For Corretto:
  - Include jmc (Java Mission Control) only for version 11
  - Exclude jmc for versions 17 and later

This ensures users get the correct tools for their specific JDK version.

## Handling Non-Standard Tools

Different approaches for vendor-specific tools:

### Approach 1: Distribution-Specific Tool Lists (Recommended)

Maintain a registry of known tools for each distribution, curated through the methods described above.

### Approach 2: Dynamic Discovery on First Use

Detect missing shims when a tool is first invoked and create them on demand.

**Note: This approach is not feasible for multi-platform implementation and will not be implemented.**

While technically possible on individual platforms, dynamic discovery faces significant cross-platform challenges:

1. **Shell-specific implementation complexity**:
   - Bash uses `command_not_found_handle` function
   - Zsh uses `command_not_found_handler` function
   - Fish shell has its own event system
   - Windows Command Prompt and PowerShell have different mechanisms
   - Each requires platform-specific code and maintenance

2. **Performance overhead**:
   - Intercepts every command-not-found error
   - Adds latency to all missing command executions
   - May interfere with other command-not-found handlers

3. **Reliability concerns**:
   - False positives when tool names match other system commands
   - Race conditions in multi-shell environments
   - Difficult to debug when issues arise
   - Unpredictable behavior across different shell configurations

4. **User experience issues**:
   - Unexpected automatic shim creation
   - No opportunity to review what tools are being added
   - Potential security concerns with automatic executable creation

This approach is documented here for completeness, but the implementation will use the more predictable and maintainable Approach 1 (Distribution-Specific Tool Lists) combined with manual shim management commands.

### Approach 3: Configuration-Based

Allow users to configure additional tools in Kopi's configuration file, providing flexibility for custom or experimental tools.

## Recommended Strategy

We use a controlled, explicit approach for shim management:

1. **Base**: Use distribution-specific tool lists (Approach 1) as the foundation
2. **Manual**: Support `kopi shim add <tool>` for users to manually add specific tools
3. **Config**: Allow configuration overrides (Approach 3) for advanced users
4. **Discovery**: Provide `kopi shim list --available` to show potential tools without automatically creating shims

### Shim Setup Process

When setting up shims, the system:

1. Starts with the standard tool set
2. Adds distribution-specific tools based on the default distribution (if configured)
3. Includes any user-configured additional tools from the configuration
4. Excludes any tools explicitly marked for exclusion in the configuration
5. Creates shim files for all resulting tools in the shim directory

This flexible approach balances automation with user control, ensuring that:

- Common tools work out of the box
- Vendor-specific tools are properly supported
- Users can customize their setup as needed
- The system remains predictable and maintainable

## Next: [Security](./12-security.md)
