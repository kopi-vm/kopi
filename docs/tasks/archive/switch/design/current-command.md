# Current Command Design

## Overview

The `kopi current` command displays the currently active JDK version and how it was resolved. This is a critical debugging and verification tool that shows users exactly which JDK version will be used when they run Java commands, and where that version setting came from.

## Command Syntax

```bash
kopi current [options]
```

### Current Implementation Status

**⚠️ This command is not yet implemented**. The current CLI in `main.rs` shows a placeholder message: "Current JDK version (not yet implemented)".

**Infrastructure Available**: The underlying `VersionResolver` in `src/version/resolver.rs` is fully implemented and can be used for this command. It already supports the complete version resolution logic described below.

### Planned Options

When implemented, this command will support:

- `-q, --quiet`: Show only the version number (no source information)
- `--json`: Output in JSON format for scripting

Verbose output will be controlled by the global `-v` flag:
- `kopi current`: Standard output (version and source)
- `kopi current -v`: Info-level logging
- `kopi current -vv`: Debug-level logging  
- `kopi current -vvv`: Trace-level logging with full resolution details

## Behavior

### Primary Function

Displays the active JDK version by following the same resolution logic as the shims:

1. Check `KOPI_JAVA_VERSION` environment variable
2. Check `.kopi-version` file in current/parent directories
3. Check `.java-version` file in current/parent directories  
4. Check global default from `~/.kopi/version`
5. Show error if no version is configured

### Output Format

#### Standard Output
```
<version> (set by <source>)
```


#### JSON Output
```json
{
  "version": "17.0.5",
  "source": ".kopi-version",
  "source_path": "/home/user/project/.kopi-version",
  "vendor": "temurin",
  "jdk_path": "/home/user/.kopi/jdks/temurin-17.0.5",
  "java_home": "/home/user/.kopi/jdks/temurin-17.0.5",
  "installed": true
}
```

## Implementation Details

### Command Flow

```rust
pub fn execute_current_command(quiet: bool, json: bool) -> Result<()> {
    // 1. Use the same VersionResolver as shims
    let resolver = VersionResolver::new();
    
    // 2. Resolve version with source tracking
    let (version_request, source) = match resolver.resolve_version_with_source() {
        Ok(result) => result,
        Err(KopiError::NoLocalVersion { .. }) => {
            if json {
                print_json_no_version()?;
            } else {
                eprintln!("No JDK version configured");
                eprintln!("Hint: Use 'kopi local <version>' to set a project version");
                eprintln!("      or 'kopi global <version>' to set a default");
            }
            return Err(KopiError::NoLocalVersion { 
                directory: std::env::current_dir()?.display().to_string() 
            });
        }
        Err(e) => return Err(e),
    };
    
    // 3. Check if the version is actually installed
    let repository = JdkRepository::new(&config);
    let is_installed = check_installation(&repository, &version_request)?;
    
    // 4. Format and display output
    if json {
        print_json_output(&version_request, &source, is_installed)?;
    } else if quiet {
        println!("{}", version_request.version_pattern);
    } else {
        print_standard_output(&version_request, &source, is_installed)?;
    }
    
    Ok(())
}
```

### Version Source Tracking

```rust
pub enum VersionSource {
    Environment,
    LocalKopiVersion(PathBuf),
    LocalJavaVersion(PathBuf),
    GlobalDefault(PathBuf),
}

impl Display for VersionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionSource::Environment => write!(f, "KOPI_JAVA_VERSION"),
            VersionSource::LocalKopiVersion(path) => write!(f, "{}", path.display()),
            VersionSource::LocalJavaVersion(path) => write!(f, "{}", path.display()),
            VersionSource::GlobalDefault(path) => write!(f, "{}", path.display()),
        }
    }
}
```

### Integration with VersionResolver

Extend VersionResolver to track the source:

```rust
impl VersionResolver {
    pub fn resolve_version_with_source(&self) -> Result<(VersionRequest, VersionSource)> {
        // Check environment variable
        if let Some(version) = env::var("KOPI_JAVA_VERSION").ok() {
            return Ok((
                VersionRequest::from_str(&version)?,
                VersionSource::Environment
            ));
        }
        
        // Check local version files
        let current_dir = env::current_dir()?;
        for ancestor in current_dir.ancestors() {
            // Check .kopi-version
            let kopi_version_path = ancestor.join(".kopi-version");
            if kopi_version_path.exists() {
                let content = self.read_version_file(&kopi_version_path)?;
                return Ok((
                    VersionRequest::from_str(&content)?,
                    VersionSource::LocalKopiVersion(kopi_version_path)
                ));
            }
            
            // Check .java-version
            let java_version_path = ancestor.join(".java-version");
            if java_version_path.exists() {
                let content = self.read_version_file(&java_version_path)?;
                return Ok((
                    VersionRequest::from_str(&content)?,
                    VersionSource::LocalJavaVersion(java_version_path)
                ));
            }
        }
        
        // Check global default
        if let Some(version_request) = self.get_global_default()? {
            let path = home_dir()
                .ok_or(KopiError::SystemError("Cannot determine home directory"))?
                .join(".kopi")
                .join("version");
            return Ok((version_request, VersionSource::GlobalDefault(path)));
        }
        
        // No version found
        Err(KopiError::NoLocalVersion {
            directory: current_dir.display().to_string(),
        })
    }
}
```

## User Experience

### Success Scenarios

#### With Environment Variable Set
```bash
$ export KOPI_JAVA_VERSION=17
$ kopi current
17 (set by KOPI_JAVA_VERSION)

$ kopi current -v
[INFO] Resolving JDK version...
17 (set by KOPI_JAVA_VERSION)
```

#### With Project Version File
```bash
$ cd my-project
$ kopi current
21.0.1 (set by /home/user/my-project/.kopi-version)

$ kopi current -q
21.0.1
```

#### With Global Default
```bash
$ cd /tmp
$ kopi current
11.0.2 (set by /home/user/.kopi/version)
```

### Error Scenarios

#### No Version Configured
```bash
$ kopi current
No JDK version configured
Hint: Use 'kopi local <version>' to set a project version
      or 'kopi global <version>' to set a default
```

#### Version Not Installed
```bash
$ kopi current
17.0.5 (set by .kopi-version) [NOT INSTALLED]
Warning: JDK version 17.0.5 is configured but not installed
Hint: Run 'kopi install 17.0.5' to install this version
```

#### JSON Error Output
```bash
$ kopi current --json
{
  "error": "no_version_configured",
  "message": "No JDK version configured",
  "hints": [
    "Use 'kopi local <version>' to set a project version",
    "Use 'kopi global <version>' to set a default"
  ]
}
```

## Integration Points

### Shell Scripts

The command is useful in shell scripts and prompts:

```bash
# In .bashrc/.zshrc for prompt
PS1='$(kopi current -q 2>/dev/null || echo "no-jdk") $ '

# In scripts
if kopi current -q >/dev/null 2>&1; then
    echo "JDK is configured: $(kopi current -q)"
else
    echo "No JDK configured"
fi
```

### CI/CD Pipelines

```yaml
# GitHub Actions example
- name: Check JDK version
  run: |
    kopi current --json > jdk-info.json
    cat jdk-info.json
```

### IDE Integration

IDEs can use the JSON output to detect the active JDK:

```bash
$ kopi current --json | jq -r .java_home
/home/user/.kopi/jdks/temurin-17.0.5
```

## Advanced Features

### Debug Mode

With increased verbosity using the global `-v` flag, the version resolution process is logged:

```rust
impl VersionResolver {
    pub fn resolve_version_with_source(&self) -> Result<(VersionRequest, VersionSource)> {
        // Check environment variable
        log::debug!("Checking KOPI_JAVA_VERSION environment variable...");
        if let Some(version) = env::var("KOPI_JAVA_VERSION").ok() {
            log::debug!("Found KOPI_JAVA_VERSION: {}", version);
            return Ok((
                VersionRequest::from_str(&version)?,
                VersionSource::Environment
            ));
        }
        log::debug!("KOPI_JAVA_VERSION not set");
        
        // Check local version files
        let current_dir = env::current_dir()?;
        log::debug!("Searching for version files from: {:?}", current_dir);
        
        for ancestor in current_dir.ancestors() {
            // Check .kopi-version
            let kopi_version_path = ancestor.join(".kopi-version");
            log::trace!("Checking {:?}", kopi_version_path);
            if kopi_version_path.exists() {
                log::debug!("Found .kopi-version at {:?}", kopi_version_path);
                let content = self.read_version_file(&kopi_version_path)?;
                log::debug!("Version content: {}", content);
                return Ok((
                    VersionRequest::from_str(&content)?,
                    VersionSource::LocalKopiVersion(kopi_version_path)
                ));
            }
            // Similar for .java-version...
        }
        
        // Rest of implementation...
    }
}
```

Example output:
```bash
$ kopi current -vvv
[TRACE] Checking /home/user/project/.kopi-version
[DEBUG] Found .kopi-version at /home/user/project/.kopi-version
[DEBUG] Version content: 17.0.5
[TRACE] Checking if temurin@17.0.5 is installed...
17.0.5 (set by /home/user/project/.kopi-version)
```

## Testing Strategy

### Unit Tests
- Version resolution with different sources
- Output formatting (standard, verbose, quiet, JSON)
- Error handling for missing versions

### Integration Tests
- End-to-end version resolution
- Interaction with actual file system
- Environment variable handling
- Multiple version file formats

### Test Scenarios
1. Environment variable takes precedence
2. Project version overrides global
3. Parent directory traversal
4. No version configured
5. Version configured but not installed
6. Various output formats

## Security Considerations

- Sanitize file paths in output to prevent path traversal
- Validate version strings before processing
- Don't expose sensitive information in JSON output
- Handle symlinks safely when resolving paths

## Platform-Specific Behavior

### Unix/Linux/macOS
- Standard path resolution
- Support for symlinks in paths

### Windows
- Handle both forward and backward slashes
- Account for drive letters in paths
- Junction points treated as directories

