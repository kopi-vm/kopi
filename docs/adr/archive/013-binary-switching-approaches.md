# ADR-013: Binary Switching Approaches for Version Management Tools

## Status
Proposed

## Context
Kopi needs to implement a mechanism to switch between different JDK versions seamlessly. To make an informed decision, we have analyzed how similar version management tools handle binary switching across different platforms (Windows, Mac, and Linux). This research focuses on tools like Volta, pyenv, rbenv, nvm, and others.

## Decision

### Binary Switching Approaches

After analyzing popular version managers, we identified three main approaches:

#### 1. **Shim-Based Approach**
Used by: pyenv, rbenv, Volta, nodenv

**How it works:**
- Creates lightweight executable scripts (shims) for each managed binary
- Places shim directory at the beginning of system PATH
- Shims intercept command calls and redirect to appropriate version
- Version selection based on configuration files in project hierarchy

**Implementation Details:**
```bash
# Example PATH configuration
export PATH="$HOME/.kopi/shims:$PATH"

# Shim script example (simplified)
#!/usr/bin/env bash
exec kopi exec "java" "$@"
```

**Advantages:**
- Works with any shell or program that respects PATH
- No shell-specific code required
- Transparent to other tools
- Clean separation of concerns

**Disadvantages:**
- Requires "rehashing" when new executables are installed
- Small performance overhead (~50ms per command)
- Requires creating shims for all executables

#### 2. **Shell Function/Alias Approach**
Used by: nvm, rvm

**How it works:**
- Modifies shell environment by creating functions or aliases
- Intercepts commands at shell level
- Dynamically updates PATH when switching versions
- Often hooks into shell commands like `cd`

**Implementation Details:**
```bash
# Example shell function
nvm() {
  # Logic to switch versions and update PATH
  export PATH="/path/to/node/version:$PATH"
}
```

**Advantages:**
- Can provide automatic switching without shims
- More flexible shell integration

**Disadvantages:**
- Shell-specific implementation required
- Doesn't work with non-shell invocations
- Can conflict with other shell modifications
- Requires sourcing scripts in shell profile

#### 3. **Hybrid Approach**
Used by: Volta (enhanced shims), asdf

**How it works:**
- Combines shims with intelligent version detection
- May include compiled binaries for performance
- Automatic version switching based on project context
- Enhanced metadata caching

**Implementation Details:**
- Fast compiled shims (Rust in Volta's case)
- Project configuration detection (.volta/package.json)
- Automatic version installation if missing
- Cached tool metadata

### Platform-Specific Considerations

#### Windows
- **PATH Management**: Windows uses semicolon-separated paths and has different PATH precedence rules
- **Shim Implementation**: 
  - Can use `.exe` or `.cmd` files as shims
  - Volta creates native Windows executables
  - pyenv-win fork provides Windows support
- **Registry Integration**: Some tools modify Windows registry for global PATH changes
- **Challenges**:
  - Different path separators (`\` vs `/`)
  - Case-insensitive filesystem
  - Different executable extensions (`.exe`, `.bat`, `.cmd`)

#### macOS
- **PATH Management**: Standard Unix PATH with colon separation
- **Shell Integration**: Typically uses `.bash_profile` or `.zshrc`
- **Shim Implementation**: Standard shell scripts or compiled binaries
- **Challenges**:
  - System Integrity Protection (SIP) restrictions
  - Different default shells (bash vs zsh)

#### Linux
- **PATH Management**: Standard Unix PATH with colon separation
- **Shell Integration**: Uses `.bashrc`, `.profile`, or shell-specific configs
- **Shim Implementation**: Shell scripts or compiled binaries
- **Advantages**: Most consistent and predictable platform

### Performance Comparison

| Tool | Switching Speed | Overhead per Command | Implementation Language |
|------|----------------|---------------------|------------------------|
| Volta | < 1 second | Minimal | Rust |
| pyenv | ~1-2 seconds | ~50ms | Shell/Python |
| rbenv | ~1-2 seconds | ~50ms | Shell/Ruby |
| nvm | 2-6 seconds | Minimal (shell function) | Shell |

### Volta's Enhanced Shims

Volta implements an advanced shim system with the following characteristics:

**Architecture Features:**
- Single compiled binary (`volta-shim`) serves as the shim for all tools
- Individual tool commands (node, npm, yarn) are symlinks to `volta-shim`
- Written in Rust for maximum performance (1-20ms overhead vs 50ms+ for shell scripts)

**Intelligent Version Detection:**
1. Determines which tool was called based on executable name
2. Walks up directory tree to find `package.json`
3. Reads version configuration from `volta` field
4. Routes to appropriate tool version

**Advanced Features:**
- Automatic installation of missing versions
- Special handling for `node_modules` directories
- Dynamic PATH management to prevent recursive shim calls
- Session tracking and error reporting

### Platform-Specific Shim Implementations

#### Linux Implementation

**Shell Script Shims (pyenv/rbenv style):**
```bash
#!/usr/bin/env bash
set -e
[ -n "$PYENV_DEBUG" ] && set -x

program="${0##*/}"
exec "/usr/local/bin/pyenv" exec "$program" "$@"
```

**Characteristics:**
- No file extension required
- Execute permission via `chmod +x`
- Uses `exec` to replace process (memory efficient)
- Shebang specifies interpreter

#### macOS Implementation

Similar to Linux with additional considerations:
- System Integrity Protection (SIP) compliance
- Universal Binary support for Intel and Apple Silicon
- Gatekeeper considerations for unsigned binaries

```bash
# Create universal binary
lipo -create target/x86_64-apple-darwin/release/kopi-shim \
             target/aarch64-apple-darwin/release/kopi-shim \
     -output kopi-shim-universal
```

#### Windows Implementation

**Current Issues with Script-based Shims:**
- `.bat` files break when called from other batch files without `call`
- CMake and other tools expect `.exe` files
- PowerShell scripts require execution policy changes

**Native Executable Approach (Recommended):**
```c
// Windows-specific shim.c
#include <windows.h>
#include <stdio.h>

int main(int argc, char* argv[]) {
    char* program = strrchr(argv[0], '\\');
    if (program) program++;
    else program = argv[0];
    
    char* dot = strrchr(program, '.');
    if (dot) *dot = '\0';
    
    char command[MAX_PATH];
    snprintf(command, sizeof(command), 
             "\"%s\\volta.exe\" run %s", 
             getenv("VOLTA_HOME"), program);
    
    return system(command);
}
```

### Efficient Shim Implementation for Kopi

To avoid process chain inefficiency (shim → kopi → java), implement direct execution:

**Inefficient Approach (Avoid):**
```rust
// Creates process chain: shim → kopi exec → java
let status = Command::new("kopi")
    .arg("exec")
    .arg(&tool_name)
    .args(env::args().skip(1))
    .status()
    .expect("Failed to execute kopi");
```

**Efficient Approach (Recommended):**
```rust
use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

fn main() {
    let tool_name = get_tool_name();
    let tool_path = resolve_tool_path(&tool_name).unwrap();
    
    // Unix: Replace current process with exec
    #[cfg(unix)]
    {
        let err = Command::new(&tool_path)
            .args(env::args().skip(1))
            .exec(); // Process replacement - shim disappears
        
        eprintln!("Failed to execute {}: {}", tool_path.display(), err);
        std::process::exit(1);
    }
    
    // Windows: Direct execution (no exec available)
    #[cfg(windows)]
    {
        let status = Command::new(&tool_path)
            .args(env::args().skip(1))
            .status()
            .expect("Failed to execute tool");
        
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn resolve_tool_path(tool_name: &str) -> Result<PathBuf, Box<dyn Error>> {
    // Direct resolution without calling kopi binary
    let version = find_project_version()?;
    let jdk_path = home_dir()
        .ok_or("Cannot find home directory")?
        .join(".kopi/jdks")
        .join(&version)
        .join("bin")
        .join(tool_name);
    
    #[cfg(windows)]
    let jdk_path = jdk_path.with_extension("exe");
    
    Ok(jdk_path)
}
```

**Performance Optimizations:**
```rust
// Version caching to minimize file I/O
static VERSION_CACHE: Lazy<Mutex<HashMap<PathBuf, String>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

fn resolve_version() -> Result<String> {
    // 1. Check environment variable (fastest)
    if let Ok(version) = env::var("KOPI_JAVA_VERSION") {
        return Ok(version);
    }
    
    // 2. Check memory cache
    let cwd = env::current_dir()?;
    if let Some(version) = VERSION_CACHE.lock().unwrap().get(&cwd) {
        return Ok(version.clone());
    }
    
    // 3. Search filesystem (slowest)
    let version = find_version_file(&cwd)?;
    VERSION_CACHE.lock().unwrap().insert(cwd, version.clone());
    Ok(version)
}
```

### Recommended Approach for Kopi

Based on our analysis, we recommend implementing a **shim-based approach** similar to Volta and pyenv, with the following characteristics:

1. **Shim Directory Structure**:
   ```
   ~/.kopi/
   ├── bin/           # Global Kopi binary
   ├── shims/         # Shim executables
   │   ├── java       # Symlink to kopi-shim (Unix) or java.exe (Windows)
   │   ├── javac      # Symlink to kopi-shim (Unix) or javac.exe (Windows)
   │   ├── jar        # Symlink to kopi-shim (Unix) or jar.exe (Windows)
   │   └── ...
   └── jdks/          # Installed JDKs
   ```

2. **Version Detection Hierarchy**:
   - Check environment variable `KOPI_JAVA_VERSION` (fastest)
   - Check in-memory cache for current directory
   - Check `.kopi-version` in current directory
   - Walk up directory tree looking for `.kopi-version` or `.java-version`
   - Fall back to global default version
   - Error if no Kopi-managed version is found

3. **Platform-Specific Implementation**:
   - **Unix (Linux/macOS)**: Single Rust binary with symlinks
   - **Windows**: Individual `.exe` files for each tool
   - Use `exec` on Unix to replace process entirely
   - Direct execution on Windows with proper exit code handling

4. **Performance Features**:
   - In-memory version caching
   - Environment variable override for CI/scripts
   - Compiled Rust shims for minimal overhead (1-20ms)
   - Direct execution without intermediate processes

5. **Auto-Installation Exception**:
   While the design prioritizes avoiding process chains for performance, auto-installation is handled as an exceptional case:
   - **Normal operation**: Shim directly executes the target JDK binary (no process chain)
   - **Auto-installation**: When a requested JDK is not installed and auto-install is enabled, the shim spawns the main kopi binary as a subprocess to handle the installation
   - This exception is acceptable because:
     - Auto-installation is an infrequent operation (typically happens once per JDK version)
     - The subprocess overhead is negligible compared to the time spent downloading and extracting a JDK
     - It keeps the shim binary small (< 1MB) by not including installation logic
     - Users can disable auto-installation if subprocess spawning is undesirable
   
   ```rust
   // Auto-installation subprocess call (exceptional case only)
   if auto_install_enabled && jdk_not_found {
       Command::new("kopi")
           .arg("install")
           .arg(format!("{}@{}", distribution, version))
           .arg("--auto")
           .status()?;
       // Retry finding JDK after installation
   }
   ```

## Rationale

1. **Shim-based approach** provides the best balance of compatibility and functionality
2. **Cross-platform support** is essential for JDK management
3. **Performance** matters for developer experience (Rust implementation)
4. **Automatic switching** reduces cognitive load
5. **Compatibility** with existing `.java-version` files eases migration

## Consequences

### Positive
- Works with any tool that invokes Java commands
- No shell-specific configuration required
- Fast switching with compiled shims
- Consistent behavior across platforms
- Easy to debug and understand

### Negative
- Need to maintain shims for all JDK executables
- Platform-specific shim implementations required
- Initial setup adds shim directory to PATH
- Rehashing required when installing new JDK versions

## Implementation Notes

1. **Shim Creation**: Generate shims for all executables in JDK `bin/` directory
2. **Rehashing**: Automatically update shims after JDK installation
3. **Performance**: Use caching to minimize filesystem lookups
4. **Error Handling**: Provide clear messages when versions are not found
5. **Debugging**: Include `KOPI_DEBUG` environment variable for troubleshooting

## References
- Volta Documentation: https://docs.volta.sh/
- pyenv Source Code: https://github.com/pyenv/pyenv
- rbenv Wiki: https://github.com/rbenv/rbenv/wiki
- nvm Implementation: https://github.com/nvm-sh/nvm
- "Deep dive into how pyenv actually works": https://www.mungingdata.com/python/how-pyenv-works-shims/