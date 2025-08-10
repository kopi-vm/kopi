# ADR-018: macOS JDK Bundle Structure Handling

## Status
Accepted

## Context

When extracting JDK archives on macOS, different distributions use different directory structures. This causes issues where Kopi cannot correctly locate the Java binaries after extraction, resulting in installation failures.

### Investigation Results

After analyzing multiple JDK distributions for macOS (aarch64), three distinct patterns were identified:

#### Pattern 1: macOS Bundle Structure (.app format)
JDK files are placed within a `Contents/Home/` directory structure, following the macOS application bundle convention:

- **Temurin**: `jdk-24.0.2+12/Contents/Home/`
- **TencentKona**: `jdk-21.0.8.jdk/Contents/Home/`

Example structure:
```
jdk-24.0.2+12/
└── Contents/
    ├── _CodeSignature/
    │   └── CodeResources
    ├── Home/
    │   ├── bin/
    │   ├── conf/
    │   ├── include/
    │   ├── legal/
    │   ├── lib/
    │   └── release
    ├── Info.plist
    └── MacOS/
        └── libjli.dylib
```

#### Pattern 2: Hybrid Structure (Bundle with Root Symlinks)
A special case where the JDK uses bundle structure internally but provides convenience symlinks at the root level:

- **Azul Zulu**: Root symlinks pointing to `zulu-24.jdk/Contents/Home/`

Example structure:
```
zulu24.32.13-ca-jdk24.0.2-macosx_aarch64/
├── bin -> zulu-24.jdk/Contents/Home/bin
├── conf -> zulu-24.jdk/Contents/Home/conf
├── lib -> zulu-24.jdk/Contents/Home/lib
├── include -> zulu-24.jdk/Contents/Home/include
├── jmods -> zulu-24.jdk/Contents/Home/jmods
├── release -> zulu-24.jdk/Contents/Home/release
└── zulu-24.jdk/
    └── Contents/
        ├── _CodeSignature/
        ├── Home/
        │   ├── bin/
        │   ├── conf/
        │   ├── lib/
        │   └── ...
        ├── Info.plist
        └── MacOS/
```

This hybrid approach allows the JDK to work both as a macOS bundle and with tools expecting direct structure.

#### Pattern 3: Direct Structure
JDK files (bin, conf, lib, etc.) are placed directly in the root directory:

- **Liberica**: `jdk-24.0.2-full.jdk/`

Example structure:
```
jdk-24.0.2-full.jdk/
├── bin/
├── conf/
├── include/
├── jmods/
├── legal/
├── lib/
├── LICENSE
└── release
```

### The Problem

Currently, Kopi assumes a direct structure for all platforms. When extracting a macOS bundle-structured JDK, it fails to find the Java binaries because they are located in `Contents/Home/` rather than at the root of the extracted directory.

This issue is specific to macOS and does not affect Windows or Linux distributions, which consistently use the direct structure.

### How Other Tools Handle This

After analyzing implementations from popular JDK management tools, different approaches were identified:

#### asdf-java
- Uses a vendor-specific approach on macOS
- For Zulu and Liberica: moves all contents as-is (respecting root symlinks)
- For all other distributions: explicitly moves `Contents/Home/*` to the install path
- Source: lines 190-217 in `bin/functions`

#### jabba
- Searches for `bin/java` anywhere in the extracted structure
- Normalizes all macOS JDKs to have `Contents/Home` structure
- If structure is already `Contents/Home`, preserves it
- Otherwise, restructures to create `Contents/Home`
- Source: `normalizePathToBinJava` function in `install.go`

#### GitHub Actions setup-java
- Simple post-installation check approach
- After installation, checks if `Contents/Home` exists on macOS
- If it exists, appends it to the Java path
- Source: lines 87-94 in `base-installer.ts`

#### SDKMAN
- No explicit handling in the main install script
- Relies on post-installation hooks downloaded per distribution/version
- Delegates structure handling to distribution-specific scripts

## Decision

Based on the analysis of existing tools, implement a hybrid approach that combines the strengths of each:

1. **Use asdf-java's vendor-aware approach** for known special cases (Zulu, Liberica)
2. **Apply GitHub Actions' simple detection** for the general case
3. **Validate like jabba** by searching for `bin/java` to ensure correctness

### Detection Logic

After extracting a JDK archive on macOS, check the directory structure in the following order:

1. If `bin/` exists at root (could be directory or symlink) → Use the extraction directory as JDK root
   - This handles both direct structure (Liberica) and hybrid structure (Zulu)
   - Validates by checking if `bin/java` or `bin/java.exe` exists
2. If `Contents/Home/` exists → Use it as the JDK root
   - This handles pure bundle structure (Temurin, TencentKona)
   - Validates by checking if `Contents/Home/bin/java` exists
3. If a subdirectory contains `Contents/Home/` → Use that subdirectory's `Contents/Home/` as the JDK root
   - This handles cases where the bundle is nested one level deep
   - Common with some archive formats that create an extra directory level
4. Otherwise → Search for `bin/java` recursively (like jabba) as a fallback
5. If still not found → Report an error for invalid JDK structure

### Implementation Approach

1. **Platform-specific handling**: Only perform this check on macOS platforms
2. **Extraction phase**: Detect structure immediately after archive extraction
3. **Installation phase**: Move the correct directory to the final installation location
4. **Validation**: Verify the presence of essential JDK components (bin/java) before completing installation

### Directory Structure After Installation

**Important Decision**: Kopi will **preserve the original directory structure** rather than normalizing or flattening it.

For macOS installations:
- Bundle structure (`Contents/Home`) will be maintained as-is
- Direct structure will be kept as-is
- Hybrid structure (Zulu) will be preserved with symlinks

Example final installation paths:
```
# macOS with bundle structure (Temurin)
~/.kopi/jdks/temurin-24.0.2-aarch64/
└── Contents/
    ├── Home/        # <- JAVA_HOME points here
    │   ├── bin/
    │   ├── conf/
    │   └── lib/
    ├── Info.plist
    └── MacOS/

# macOS with direct structure (Liberica)
~/.kopi/jdks/liberica-24.0.2-aarch64/  # <- JAVA_HOME points here
├── bin/
├── conf/
└── lib/

# Linux/Windows (always direct)
~/.kopi/jdks/temurin-24.0.2-x64/  # <- JAVA_HOME points here
├── bin/
├── conf/
└── lib/
```

This approach:
- Preserves code signing and notarization
- Maintains compatibility with `/usr/libexec/java_home` (future feature)
- Respects the distribution vendor's intended structure
- Simplifies implementation by avoiding complex transformations

### Code Location

The implementation will primarily affect:
- `src/archive/mod.rs` - Archive extraction logic
- `src/commands/install.rs` - Installation process
- `src/platform/file_ops.rs` - Platform-specific file operations

## Consequences

### Positive
- **Compatibility**: Supports all major JDK distributions on macOS
- **Transparency**: Users don't need to know about underlying structure differences
- **Reliability**: Prevents installation failures for bundle-structured JDKs
- **Future-proof**: Can easily extend to handle other structure variations

### Negative
- **Complexity**: Adds platform-specific logic to extraction process
- **Maintenance**: Must track changes in distribution packaging formats
- **Testing**: Requires testing with multiple JDK distributions on macOS

### Neutral
- **Performance**: Minimal impact - only adds one directory check
- **Other platforms**: No change to Windows/Linux behavior
- **Backward compatibility**: Existing installations remain unaffected

## Shim and Env Command Behavior

### Shim Operation

The **kopi-shim** (e.g., `~/.kopi/shims/java`) transparently handles the directory structure differences:

1. **Version Resolution**
   - Searches for `.kopi-version` or `.java-version` from current directory upwards
   - Falls back to global settings, then system defaults if not found

2. **JDK Path Resolution**
   - Determines installed JDK path from the resolved version
   - Example: `temurin@21` → `~/.kopi/jdks/temurin-21.0.2-aarch64/`

3. **JAVA_HOME Adjustment (macOS-specific)**
   - On macOS: Checks for `Contents/Home` directory existence
   - If exists: Uses `~/.kopi/jdks/temurin-21.0.2-aarch64/Contents/Home` as JAVA_HOME
   - If not: Uses base directory directly (for distributions like Liberica)
   - On Linux/Windows: Always uses base directory

4. **Execution**
   - Sets `JAVA_HOME` environment variable to the adjusted path
   - Executes corresponding Java binary (`$JAVA_HOME/bin/java`) with passed arguments

### Env Command Operation

The **`kopi env`** command outputs shell-appropriate environment variables:

1. **Context Detection**
   - Determines effective Java version for current directory
   - Uses same resolution logic as shim

2. **Path Resolution and Adjustment**
   - Gets base path of installed JDK
   - On macOS: Appends `Contents/Home` if it exists
   - On Linux/Windows: Uses base path as-is

3. **Environment Variable Generation**
   - `JAVA_HOME`: Adjusted JDK home directory
   - `PATH`: Prepends `$JAVA_HOME/bin`
   - Optional: `JDK_HOME`, `JRE_HOME` if needed

4. **Output Format**
   - Bash/Zsh: `export JAVA_HOME=/path/to/jdk`
   - Fish: `set -x JAVA_HOME /path/to/jdk`
   - PowerShell: `$env:JAVA_HOME="/path/to/jdk"`

### Practical Examples

**macOS + Temurin (Bundle Structure):**
- Installation: `~/.kopi/jdks/temurin-21.0.2-aarch64/Contents/Home/bin/java`
- JAVA_HOME set by shim: `~/.kopi/jdks/temurin-21.0.2-aarch64/Contents/Home`
- Executed binary: `~/.kopi/jdks/temurin-21.0.2-aarch64/Contents/Home/bin/java`

**macOS + Liberica (Direct Structure):**
- Installation: `~/.kopi/jdks/liberica-21.0.2-aarch64/bin/java`
- JAVA_HOME set by shim: `~/.kopi/jdks/liberica-21.0.2-aarch64`
- Executed binary: `~/.kopi/jdks/liberica-21.0.2-aarch64/bin/java`

**Linux + Any Distribution (Always Direct):**
- Installation: `~/.kopi/jdks/temurin-21.0.2-x64/bin/java`
- JAVA_HOME set by shim: `~/.kopi/jdks/temurin-21.0.2-x64`
- Executed binary: `~/.kopi/jdks/temurin-21.0.2-x64/bin/java`

### Key Design Principles

1. **Transparency**: Users never need to know about `Contents/Home` existence
2. **Compatibility**: Provides correct `JAVA_HOME` expected by IDEs and build tools
3. **Platform Awareness**: macOS-specific handling is automatic and invisible
4. **Performance**: Structure information is stored in metadata files to minimize filesystem checks
5. **Consistency**: Same user experience across all platforms despite structural differences

### Performance Optimization Strategy

To avoid repeated filesystem checks for directory structure, Kopi leverages its existing metadata system with extensions for macOS bundle structure:

**Existing Infrastructure:**
Kopi already has a `save_jdk_metadata` function in `src/storage/mod.rs` that saves Package information from the API. This will be extended to include local structure information:

**Extended Metadata Storage:**
```json
// ~/.kopi/jdks/temurin-21.0.2-aarch64.meta.json
{
  // Existing Package fields from API
  "id": "package-id",
  "distribution": "temurin",
  "java_version": "21.0.2",
  "distribution_version": "21.0.2+13",
  
  // New fields for local installation
  "installation_metadata": {
    "java_home_suffix": "Contents/Home",  // Empty string for direct structure
    "has_bundle_structure": true,
    "structure_type": "bundle",  // "bundle", "direct", or "hybrid"
    "detected_at": "2025-08-10T10:30:00Z",
    "platform": "macos_aarch64"
  }
}
```

**Integration Points:**
1. **During Installation (`InstallationContext`):**
   - Detect structure after extraction
   - Add `installation_metadata` to the Package data
   - Save using existing `save_jdk_metadata` function

2. **Runtime Resolution (`InstalledJdk` enhancement):**
   - Load metadata file if it exists
   - Cache the `java_home_suffix` in memory for the shim process
   - Fall back to directory detection if metadata is missing

3. **Backward Compatibility:**
   - If metadata file exists without `installation_metadata`, detect structure on first use
   - Update metadata file with detected structure for future use

This approach:
- Reuses existing metadata infrastructure
- Maintains compatibility with API data
- Provides fast runtime resolution
- Supports gradual migration of existing installations

## Implementation Notes

1. **Structure Detection Function** (incorporating lessons from other tools):
```rust
fn detect_jdk_root(extracted_dir: &Path, distribution: &str) -> Result<PathBuf> {
    // Special handling for known distributions (like asdf-java)
    if cfg!(target_os = "macos") {
        match distribution {
            "zulu" | "liberica" => {
                // These distributions handle their own structure correctly
                if extracted_dir.join("bin").exists() {
                    return validate_jdk_root(extracted_dir);
                }
            }
            _ => {}
        }
    }
    
    // Check for direct structure or hybrid (Zulu-style with symlinks)
    if extracted_dir.join("bin").exists() {
        return validate_jdk_root(extracted_dir);
    }
    
    // Check for macOS bundle structure at root (like GitHub Actions)
    let bundle_home = extracted_dir.join("Contents").join("Home");
    if bundle_home.exists() {
        return validate_jdk_root(&bundle_home);
    }
    
    // Check for nested bundle structure (e.g., jdk-x.y.z.jdk/Contents/Home/)
    for entry in fs::read_dir(extracted_dir)? {
        let entry = entry?;
        let nested_bundle = entry.path().join("Contents").join("Home");
        if nested_bundle.exists() {
            if let Ok(path) = validate_jdk_root(&nested_bundle) {
                return Ok(path);
            }
        }
    }
    
    // Fallback: search for bin/java recursively (like jabba)
    if let Some(java_path) = find_java_binary(extracted_dir)? {
        // Found java binary, return the JDK root (2 levels up from bin/java)
        if let Some(jdk_root) = java_path.parent().and_then(|p| p.parent()) {
            return validate_jdk_root(jdk_root);
        }
    }
    
    // No valid JDK structure found
    Err(KopiError::InvalidJdkStructure)
}

fn validate_jdk_root(path: &Path) -> Result<PathBuf> {
    let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
    let java_path = path.join("bin").join(java_binary);
    
    if java_path.exists() {
        Ok(path.to_path_buf())
    } else {
        Err(KopiError::InvalidJdkStructure)
    }
}

fn find_java_binary(dir: &Path) -> Result<Option<PathBuf>> {
    let java_binary = if cfg!(windows) { "java.exe" } else { "java" };
    
    for entry in walkdir::WalkDir::new(dir).max_depth(4) {
        let entry = entry?;
        if entry.file_type().is_file() 
            && entry.file_name() == java_binary 
            && entry.path().parent().and_then(|p| p.file_name()) == Some(OsStr::new("bin")) {
            return Ok(Some(entry.path().to_path_buf()));
        }
    }
    Ok(None)
}
```

2. **Platform Conditional**:
   - Only apply this logic when `cfg!(target_os = "macos")`
   - Windows and Linux continue using current extraction logic

3. **Validation**:
   - After determining JDK root, verify `bin/java` exists
   - Check for other essential components (lib, conf directories)

4. **Edge Cases**:
   - Handle nested archive structures (tar within zip)
   - Support Azul Zulu's hybrid approach (symlinks at root pointing to bundle structure)
   - Preserve code signing and notarization attributes
   - Ensure symlinks are followed correctly when validating JDK structure

5. **Testing Recommendations** (based on analysis of other tools):
   - Test with at least these distributions on macOS:
     - **Temurin**: Pure bundle structure (`Contents/Home/`)
     - **Azul Zulu**: Hybrid structure (symlinks at root + bundle)
     - **Liberica**: Direct structure (no bundle)
     - **TencentKona**: Bundle structure
     - **Corretto**: Should verify which structure it uses
   - Test both `.tar.gz` and `.zip` formats where available
   - Verify that symlinks are preserved and work correctly
   - Ensure code signing is maintained (important for macOS Gatekeeper)

## Key Insights from Other Tools

1. **No single approach works for all distributions** - Each tool has evolved different strategies
2. **Vendor-specific handling is sometimes necessary** - Zulu and Liberica are special cases
3. **Validation is critical** - Always verify `bin/java` exists after detection
4. **Flexibility is important** - Having fallback detection methods improves robustness
5. **macOS integration** - Some tools (asdf-java) also handle `/usr/libexec/java_home` integration

## Implementation Phases

### Phase 1: Basic Structure Detection (MVP)
1. Implement `detect_jdk_root` function for macOS
2. Update installation process to handle different structures
3. Modify shim to check for `Contents/Home` at runtime
4. Basic testing with major distributions

### Phase 2: Metadata Integration
1. Extend `save_jdk_metadata` to include `installation_metadata`
2. Update `InstalledJdk` to read metadata files
3. Implement metadata caching in shim
4. Migration path for existing installations

### Phase 3: Advanced Features (Future)
1. `/usr/libexec/java_home` integration (separate ADR)

## References

- [Apple Developer Documentation - Bundle Structures](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html)
- [AdoptOpenJDK/Temurin macOS packaging](https://github.com/adoptium/temurin-build)
- [Azul Zulu macOS distribution format](https://www.azul.com/downloads/)
- [asdf-java implementation](https://github.com/halcyon/asdf-java)
- [jabba implementation](https://github.com/shyiko/jabba)
- [GitHub Actions setup-java](https://github.com/actions/setup-java)
- [SDKMAN CLI](https://github.com/sdkman/sdkman-cli)