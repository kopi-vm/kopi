# Distribution-Specific Tools Investigation Report

## Investigation Date
2025-07-08 (Updated: 2025-07-09 - Additional BellSoft Liberica attempts)

## Purpose
To verify whether the current implementation of `discover_distribution_tools` in `/src/shim/discovery.rs` correctly handles distribution-specific tools, and to identify any additional distributions that may require special handling.

## Methodology

1. Installed multiple JDK distributions (version 21 for consistency)
2. Examined the contents of each distribution's `bin` directory
3. Compared tools against the standard JDK tools list in `/src/shim/tools.rs`
4. Identified distribution-specific tools

## Distributions Investigated

### Successfully Installed and Analyzed
- **Eclipse Temurin** 21.0.7
- **Amazon Corretto** 21.0.7.6.1
- **Azul Zulu** 21.42.19
- **Alibaba Dragonwell** 21.0.7.0.7.6
- **IBM Semeru** 21.0.7
- **GraalVM** 21.0.7 (re-attempted successfully on 2025-07-09)
- **SAP Machine** 21.0.7 (installed on 2025-07-09)
- **BellSoft Liberica** 21.0.7+9 (installed on 2025-07-09 after checksum fix)


## Findings

### 1. GraalVM Special Handling (Updated)

Investigation of GraalVM 21.0.7 revealed:
- `native-image` is present ✓
- `gu` is NOT present in GraalVM 21.0.7 (likely removed in recent versions)
- `js` is NOT present in GraalVM 21.0.7 (consistent with tools.rs which notes it was removed in version 23+)

GraalVM 21.0.7 includes three native-image related tools:
- `native-image` - Native Image compiler
- `native-image-configure` - Configuration tool for native image builds
- `native-image-inspect` - Inspection tool for native images

**Status**: ✅ The implementation has been updated:
- Removed `gu` from the GraalVM tool check
- Added all three native-image tools to both `discover_distribution_tools` and the tool registry

### 2. Missing Standard JDK Tools (Added)

The following tools were found in ALL investigated distributions but were NOT initially listed in `/src/shim/tools.rs`:

```
jdeprscan    - Deprecated API scanner (min_version: 9)
jhsdb        - HotSpot Debugger (min_version: 9)
jimage       - JDK module image tool (min_version: 9)
jrunscript   - Script execution tool
jstatd       - JSTAT daemon
jwebserver   - Simple web server (min_version: 18)
rmiregistry  - RMI registry
```

**Status**: ✅ All seven tools have been added to the tool registry with appropriate version constraints and categories.

### 3. IBM Semeru Contains OpenJ9-Specific Tools (Implemented)

**IBM Semeru** includes four distribution-specific tools not found in HotSpot-based JDKs:
- **jdmpview** - Java dump viewer for analyzing system dumps
- **jitserver** - JIT compilation server for offloading JIT compilation
- **jpackcore** - Tool for packaging core dumps
- **traceformat** - Tool for formatting trace files

These tools are specific to the OpenJ9 VM that Semeru uses.

**Status**: ✅ The implementation has been updated to handle Semeru/OpenJ9 distributions and discover these four specific tools.

### 4. SAP Machine Contains SAP-Specific Tools (Implemented)

**SAP Machine** 21.0.7 includes one distribution-specific tool:
- **asprof** - SAP's profiler tool (async-profiler based)

This tool is specific to SAP Machine distribution and provides advanced profiling capabilities.

**Status**: ✅ The implementation has been updated to handle SAP Machine distributions and discover the `asprof` tool.

### 5. Other Distributions

- **Alibaba Dragonwell**: No distribution-specific tools found (contains only standard JDK tools)
- **Temurin, Corretto, Zulu**: No vendor-specific tools discovered beyond the standard JDK toolset

## Implementation Status

Based on the investigation findings, the following changes have been implemented:

1. **✅ Updated GraalVM Special Handling**: 
   - Removed `gu` from the GraalVM tool check
   - Added `native-image`, `native-image-configure`, and `native-image-inspect` to discovery
   - All three tools added to the tool registry

2. **✅ Added IBM Semeru/OpenJ9 Special Handling**: 
   - `discover_distribution_tools` now recognizes Semeru's four specific tools
   - Handles both "semeru" and "openj9" distribution names

3. **✅ Added SAP Machine Special Handling**: 
   - `discover_distribution_tools` now recognizes SAP Machine's specific tool
   - Added `asprof` to the tool registry with proper distribution exclusions
   - Handles both "sap_machine" and "sapmachine" distribution names

4. **✅ Updated Standard Tools Registry**: 
   - Added all seven missing standard tools to `/src/shim/tools.rs`
   - Applied appropriate version constraints (e.g., `jwebserver` min_version: 18)
   - Categorized tools appropriately

## BellSoft Liberica Investigation (Completed 2025-07-09)

### Checksum Verification Fix
The checksum mismatch issue has been resolved. The `security` module now supports multiple checksum algorithms (SHA1, SHA256, SHA512, MD5) as implemented in commit 8ca70e7 "チェックサムをバリデーションするアルゴリズムを選択可能にした。"

### Investigation Results
**BellSoft Liberica** 21.0.7+9 was successfully installed and analyzed:
- Contains ONLY standard JDK tools
- No distribution-specific tools found
- No NIK (Native Image Kit) found in the standard distribution
- No enhanced diagnostic tools detected

The complete list of tools in Liberica 21.0.7+9 matches the standard JDK toolset exactly:
- jar, jarsigner, java, javac, javadoc, javap, jcmd, jconsole, jdb
- jdeprscan, jdeps, jfr, jhsdb, jimage, jinfo, jlink, jmap, jmod
- jpackage, jps, jrunscript, jshell, jstack, jstat, jstatd, jwebserver
- keytool, rmiregistry, serialver

### Notes
- The NIK (Native Image Kit) may be available in special Liberica distributions, but not in the standard releases available through foojay.io
- No additional configuration is needed for `discover_distribution_tools` as Liberica contains no unique tools

## Future Recommendations

1. **Distribution Investigations Still Needed**: 
   - Red Hat Mandrel (GraalVM derivative, may have unique tools)
   - OpenJDK
   - Trava OpenJDK
   - Tencent Kona

2. **Implementation Tasks**:
   - ✅ COMPLETED: Added SAP Machine support to `discover_distribution_tools` for the `asprof` tool
   - ✅ COMPLETED: Fixed checksum verification to support multiple algorithms (SHA1, SHA256, SHA512, MD5)
     - Updated `src/security/mod.rs` to handle different checksum types
     - Now uses the `checksum_type` field from JdkMetadata when verifying downloads
     - Successfully unblocked BellSoft Liberica installation and investigation

3. **Potential Enhancements**: 
   - Dynamic discovery of non-standard tools
   - Metadata about tool availability per distribution version
   - Caching of discovered tools per installation

## Technical Notes

- The investigation used the command pattern: `cargo run --bin kopi -- install <distribution>@21`
- Tool lists were generated by comparing directory listings against the standard tools extracted from `tools.rs`
- GraalVM 21.0.7 was successfully installed after initial failures (2025-07-09)
- The absence of `gu` in GraalVM 21.0.7 suggests Oracle has changed the GraalVM distribution model
- SAP Machine 21.0.7 includes `asprof`, a distribution-specific profiler tool based on async-profiler
- BellSoft Liberica 21.0.7+9 was successfully installed after checksum verification fix (2025-07-09)
- Investigation revealed foojay.io provides SHA1 checksums for Liberica, which is now properly handled
- Liberica contains only standard JDK tools with no distribution-specific additions

## Conclusion

The investigation successfully identified distribution-specific tools and missing standard JDK tools. All findings have been implemented:

1. **GraalVM** - Implementation updated:
   - Removed check for `gu` (confirmed absent in GraalVM 21.0.7)
   - Now checks for three native-image tools: `native-image`, `native-image-configure`, and `native-image-inspect`
   - All tools properly added to the registry with distribution exclusions
   
2. **IBM Semeru** - OpenJ9-specific tools implemented:
   - Four tools are now discovered: `jdmpview`, `jitserver`, `jpackcore`, `traceformat`
   - Handles both "semeru" and "openj9" distribution identifiers

3. **SAP Machine** - Distribution-specific tool implemented:
   - Contains `asprof` profiler tool
   - ✅ Now properly discovered by `discover_distribution_tools`
   - Handles both "sap_machine" and "sapmachine" distribution identifiers

4. **Standard JDK Tools** - Seven missing tools added to the registry:
   - All tools categorized and versioned appropriately
   - Version constraints applied where necessary (e.g., `jwebserver` for Java 18+)

5. **BellSoft Liberica** - Investigation completed:
   - Successfully installed and analyzed version 21.0.7+9
   - Root cause of initial failure: BellSoft Liberica provides SHA1 checksums via foojay.io API
   - Fixed: kopi now supports multiple checksum algorithms (SHA1, SHA256, SHA512, MD5)
   - Investigation result: Liberica contains only standard JDK tools
   - No distribution-specific tools found, no special handling required

The implementation now correctly handles all known distribution-specific tools (GraalVM, IBM Semeru, and SAP Machine) and includes a comprehensive registry of standard JDK tools.