# Local Command Design

## Overview

The `kopi local` command (alias: `kopi pin`) sets a project-specific JDK version by creating a `.kopi-version` file in the current directory. This version is automatically activated when entering the project directory.

## Command Syntax

```bash
kopi local <version>
kopi pin <version>     # Alias
```

### Arguments

- `<version>`: The JDK version to set for the project
  - Format: `<major>`, `<major.minor.patch>`, or `<distribution>@<version>`
  - Examples: `17`, `17.0.5`, `temurin@17.0.5`

## Behavior

### Primary Function

Creates or updates a `.kopi-version` file in the current directory containing the specified version. The shims automatically detect and use this version when executing Java commands from this directory or its subdirectories.

If the specified version is not installed:
- **Auto-install enabled**: Prompts the user to install the JDK
- **Auto-install disabled**: Creates the version file with a warning
- **User declines**: Creates the version file anyway (for team collaboration)

This allows teams to share version requirements via `.kopi-version` files without forcing immediate installation.

### File Format

The `.kopi-version` file is a simple text file:
```
17.0.5
```

Or with distribution:
```
temurin@17.0.5
```

### Directory Traversal

When resolving versions, shims search for version files by traversing up the directory tree:
1. Current directory
2. Parent directory
3. Continue until root or version file found

## Implementation Details

### Command Flow

```rust
use crate::shim::auto_install::AutoInstaller;
use crate::storage::JdkRepository;
use crate::models::version::VersionRequest;

pub fn execute_local_command(version: &str) -> Result<()> {
    // 1. Parse and validate version
    let version_request = VersionRequest::from_str(version)?;
    let resolved_version = resolve_version(&version_request)?;
    
    // 2. Check if version is installed
    let repository = JdkRepository::new(&config);
    if !repository.is_installed(&resolved_version)? {
        // Use AutoInstaller for handling missing versions
        let auto_installer = AutoInstaller::new(&config);
        
        if auto_installer.should_auto_install() {
            // Prompt user if configured
            let version_spec = if let Some(dist) = &version_request.distribution {
                format!("{}@{}", dist, version_request.version_pattern)
            } else {
                version_request.version_pattern.clone()
            };
            
            if auto_installer.prompt_user(&version_spec)? {
                // Install the JDK
                auto_installer.install_jdk(&version_request)?;
                println!("Successfully installed {}", version_spec);
            } else {
                // User declined installation
                eprintln!("Warning: JDK version '{}' is not installed", version);
                eprintln!("The .kopi-version file will be created, but the JDK needs to be installed manually");
                eprintln!("Run 'kopi install {}' to install it later", version);
            }
        } else {
            // Auto-install disabled
            eprintln!("Warning: JDK version '{}' is not installed", version);
            eprintln!("The .kopi-version file will be created, but the JDK needs to be installed manually");
            eprintln!("Run 'kopi install {}' to install it", version);
            eprintln!("Or enable auto-installation in ~/.kopi/config.toml");
        }
    }
    
    // 3. Check for existing version file
    let version_file = Path::new(".kopi-version");
    if version_file.exists() {
        // Read existing version for comparison
        let existing = fs::read_to_string(&version_file)?;
        eprintln!("Updating .kopi-version from {} to {}", existing.trim(), resolved_version);
    }
    
    // 4. Write version file
    fs::write(&version_file, format!("{}\n", resolved_version))?;
    
    // 5. Provide feedback
    println!("Set project JDK version to {}", resolved_version);
    println!("Created .kopi-version in {}", env::current_dir()?.display());
    
    Ok(())
}
```

### Version File Discovery

Used by shims to find project version:

```rust
pub fn find_local_version() -> Option<String> {
    let mut current = env::current_dir().ok()?;
    
    loop {
        // Check for .kopi-version
        let kopi_version = current.join(".kopi-version");
        if kopi_version.exists() {
            if let Ok(version) = fs::read_to_string(&kopi_version) {
                return Some(version.trim().to_string());
            }
        }
        
        // Check for .java-version (compatibility)
        let java_version = current.join(".java-version");
        if java_version.exists() {
            if let Ok(version) = fs::read_to_string(&java_version) {
                return Some(version.trim().to_string());
            }
        }
        
        // Move to parent directory
        if !current.pop() {
            break;
        }
    }
    
    None
}
```

## User Experience

### Success Scenarios

#### Creating New Version File
```bash
$ cd my-project
$ kopi local 17
Set project JDK version to 17.0.5
Created .kopi-version in /home/user/my-project

$ cat .kopi-version
17.0.5
```

#### Updating Existing Version
```bash
$ kopi local 21
Updating .kopi-version from 17.0.5 to 21.0.1
Set project JDK version to 21.0.1
Created .kopi-version in /home/user/my-project
```

### Error Scenarios

#### Version Not Installed (Auto-Install Enabled)
```bash
$ kopi local 19
JDK 19 is not installed. Would you like to install it now? [Y/n] y
[kopi] Installing JDK 19...
[=====================================>] 100%
Successfully installed 19.0.2
Set project JDK version to 19.0.2
Created .kopi-version in /home/user/my-project
```

#### Version Not Installed (User Declines)
```bash
$ kopi local 19
JDK 19 is not installed. Would you like to install it now? [Y/n] n
Warning: JDK version '19' is not installed
The .kopi-version file will be created, but the JDK needs to be installed manually
Run 'kopi install 19' to install it later
Set project JDK version to 19.0.2
Created .kopi-version in /home/user/my-project
```

#### Version Not Installed (Auto-Install Disabled)
```bash
$ kopi local 19
Warning: JDK version '19' is not installed
The .kopi-version file will be created, but the JDK needs to be installed manually
Run 'kopi install 19' to install it
Or enable auto-installation in ~/.kopi/config.toml
Set project JDK version to 19.0.2
Created .kopi-version in /home/user/my-project
```

#### Invalid Version Format
```bash
$ kopi local invalid-version
Error: Invalid version format 'invalid-version'
Hint: Use format like '17', '17.0.5', or 'temurin@17'
```

#### Permission Denied
```bash
$ kopi local 17
Error: Permission denied: cannot create .kopi-version
Hint: Check write permissions for current directory
```

## Integration Features

### Git Integration

The `.kopi-version` file should be committed to version control:

```bash
$ echo '.kopi-version' >> .gitignore  # Don't do this!
$ git add .kopi-version               # Do this instead
$ git commit -m "Set project JDK to version 17"
```

### Compatibility with .java-version

Kopi reads `.java-version` files for compatibility with other tools:
- Priority: `.kopi-version` > `.java-version`
- Format differences are handled transparently
- Migration path for projects using other tools

### IDE Integration

IDEs can read the `.kopi-version` file to configure project SDKs:
- IntelliJ IDEA: Via plugin or manual configuration
- VS Code: Through Java extension settings
- Eclipse: Project-specific compiler settings

## Advanced Features

### Version File Validation

```rust
fn validate_version_file(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.is_empty() {
        return Err(KopiError::EmptyVersionFile);
    }
    
    if lines.len() > 1 {
        eprintln!("Warning: Multiple lines in version file, using first line only");
    }
    
    let version = lines[0].trim();
    parse_version_string(version)?;
    
    Ok(())
}
```

### Recursive Project Discovery

Option to set version for parent project:

```bash
$ kopi local --recursive 17
Searching for project root...
Found .git directory at /home/user/my-project
Created .kopi-version in /home/user/my-project
```

## Testing Strategy

### Unit Tests
- Version file creation and updates
- Directory traversal logic
- Version format validation
- File permission handling

### Integration Tests
- Shim integration with version files
- Multi-level directory traversal
- Compatibility with .java-version
- Concurrent file access

### Scenarios to Test
1. Create new version file
2. Update existing version file
3. Handle read-only directories
4. Version file in parent directories
5. Both .kopi-version and .java-version present

## Security Considerations

- Validate version strings to prevent path injection
- Check file permissions before writing
- Don't follow symlinks when traversing directories
- Limit directory traversal depth to prevent DoS

## Future Enhancements

1. **Template Support**: Create from project templates with preset versions
2. **Version Constraints**: Support version ranges (e.g., `>=17`)
3. **Multiple Versions**: Support for different tools (javac vs java)
4. **Project Profiles**: Different versions for dev/test/prod environments