# Handling Distribution-Specific Tools

## The Challenge

Different JDK distributions include vendor-specific tools:

- **GraalVM**: `native-image`, `gu`, `polyglot`
- **OpenJ9**: `traceformat`, `jextract`
- **Corretto**: `jmc` (in some versions)

When a user tries to execute a tool that exists in one distribution but not in another, the shim provides a clear error message.

## Implementation Strategy

### Tool Resolution Process

When a shim is invoked for a specific tool, the system follows these steps:

1. **Determine the active JDK version** based on project configuration or global settings
2. **Check if the tool exists** in the active JDK's bin directory
3. **If the tool is missing**, search all installed JDKs to find which distributions include it
4. **Provide helpful guidance** to the user about how to access the tool

### Error Handling for Missing Tools

When a distribution-specific tool is not available in the current JDK, the shim provides detailed information:

- If the tool exists in other installed JDKs, it lists which distributions have it
- If the tool doesn't exist in any installed JDK, it informs the user accordingly
- It suggests actionable steps like switching JDK versions or installing a compatible distribution

### Tool Discovery Mechanism

The system maintains awareness of which tools are available in each installed JDK by:

- Scanning the bin directory of each JDK installation
- Building a map of tool availability across distributions
- Using this information to provide intelligent error messages

## User Experience Example

```bash
# Scenario: GraalVM installed, project uses Temurin
$ cat .kopi-version
temurin@21

$ native-image --version
kopi: Tool 'native-image' is not available in temurin 21.0.1
This tool is available in:
  - graalvm 21.0.2

To use this tool, either:
  1. Switch to a project using one of the above JDKs
  2. Run 'kopi use graalvm@21' to temporarily switch
  3. Install a JDK that includes this tool

# Scenario: Tool doesn't exist in any installed JDK
# Note: This only happens if a shim for 'some-unknown-tool' was previously created
$ some-unknown-tool
kopi: Tool 'some-unknown-tool' is not available in any installed JDK
```

## Distribution-Specific Tool Registry

### Known Distribution-Specific Tools

The system maintains a registry of tools that are specific to certain JDK distributions:

**GraalVM-specific tools:**

- `gu` - GraalVM Updater for managing GraalVM components
- `native-image` - Ahead-of-time compilation to native executables
- `polyglot` - Launcher for polyglot applications
- `lli` - LLVM bitcode interpreter
- `js` - JavaScript launcher
- `node` - Node.js runtime (when installed as a GraalVM component)

**OpenJ9-specific tools:**

- `traceformat` - Tool for formatting trace files
- `jextract` - Dump file extraction utility

**Corretto-specific tools:**

- `jmc` - JDK Mission Control (available in some Corretto versions)

### Standard JDK Tools

All JDK distributions include the standard tools:

- Core tools: `java`, `javac`, `jar`
- Documentation tools: `javadoc`, `javap`
- Debugging tools: `jdb`, `jconsole`
- Security tools: `keytool`, `jarsigner`
- Performance tools: `jfr`, `jcmd`, `jinfo`, `jmap`, `jps`, `jstack`, `jstat`, `jstatd`
- Modern tools: `jshell`, `jlink`, `jmod`, `jdeps`
- Other utilities: `rmiregistry`, `serialver`

## Implications

1. **Tool Availability**: A shim existing doesn't guarantee the tool is available in the current JDK
2. **Error Handling**: Shims must gracefully handle missing tools with helpful messages
3. **User Education**: Clear documentation about which tools belong to which distributions
4. **Project Portability**: Projects using distribution-specific tools may not be portable

## Next: [Shim Installation and Management](./10-shim-installation-management.md)
