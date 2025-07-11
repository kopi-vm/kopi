# Global Command Design

## Overview

The `kopi global` command (alias: `kopi default`) sets the system-wide default JDK version. This is the lowest priority setting, used when no shell or project-specific version is configured.

## Command Syntax

```bash
kopi global <version>
kopi default <version>     # Alias
```

### Arguments

- `<version>`: The JDK version to set as global default
  - Format: `<major>`, `<major.minor.patch>`, or `<distribution>@<version>`
  - Examples: `17`, `17.0.5`, `temurin@17.0.5`

## Behavior

### Primary Function

Updates the global version configuration file at `~/.kopi/version`. This file is read by shims as the fallback when no other version is specified.

Unlike `local` command, `global` command requires the version to be installed before setting it as the default:
- **Auto-install enabled**: Prompts the user to install the JDK if not present
- **Auto-install disabled**: Returns an error if the version is not installed
- **User declines**: Returns an error without changing the global version

This ensures the global default is always a working JDK installation.

### File Location and Format

Global version file: `~/.kopi/version`

Content format:
```
17.0.5
```

Or with distribution:
```
temurin@17.0.5
```

### System Integration

The global version serves as the default for:
- New shell sessions without KOPI_JAVA_VERSION
- Directories without .kopi-version or .java-version
- System services and cron jobs
- IDE default configurations

## Implementation Details

**Note**: Current implementation uses `~/.kopi/default-version` but should be changed to `~/.kopi/version` to match this design specification and align with other version management tools like rbenv and pyenv.

### Command Flow

```rust
use crate::shim::auto_install::AutoInstaller;
use crate::storage::JdkRepository;
use crate::models::version::VersionRequest;

pub fn execute_global_command(version: &str) -> Result<()> {
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
                return Err(KopiError::VersionNotInstalled(version.to_string()));
            }
        } else {
            // Auto-install disabled
            return Err(KopiError::VersionNotInstalled(version.to_string()));
        }
    }
    
    // 3. Get Kopi home directory
    let kopi_home = get_kopi_home()?;
    let version_file = kopi_home.join("version");
    
    // 4. Check existing global version
    if version_file.exists() {
        let existing = fs::read_to_string(&version_file)?;
        eprintln!("Updating global JDK from {} to {}", existing.trim(), resolved_version);
    }
    
    // 5. Ensure directory exists
    fs::create_dir_all(&kopi_home)?;
    
    // 6. Write version file
    fs::write(&version_file, format!("{}\n", resolved_version))?;
    
    // 7. Update system JDK symlinks if needed
    update_system_symlinks(&resolved_version)?;
    
    // 8. Provide feedback
    println!("Set global JDK version to {}", resolved_version);
    println!("This will be the default for new shells and projects without local versions");
    
    Ok(())
}
```

### System Symlink Management

```rust
fn update_system_symlinks(version: &str) -> Result<()> {
    let kopi_home = get_kopi_home()?;
    let default_link = kopi_home.join("default");
    
    // Remove existing symlink
    if default_link.exists() {
        fs::remove_file(&default_link).ok();
    }
    
    // Create new symlink to versioned JDK
    let jdk_path = kopi_home.join("jdks").join(version);
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&jdk_path, &default_link)?;
    }
    
    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        symlink_dir(&jdk_path, &default_link)?;
    }
    
    Ok(())
}
```

### Global Version Resolution

Used by shims as fallback:

```rust
pub fn get_global_version() -> Option<String> {
    let kopi_home = get_kopi_home().ok()?;
    let version_file = kopi_home.join("version");
    
    if version_file.exists() {
        fs::read_to_string(&version_file)
            .ok()
            .map(|v| v.trim().to_string())
    } else {
        None
    }
}
```

## User Experience

### Success Scenarios

#### Setting Initial Global Version
```bash
$ kopi global 17
Set global JDK version to 17.0.5
This will be the default for new shells and projects without local versions

$ kopi current
17.0.5 (set by ~/.kopi/version)
```

#### Updating Global Version
```bash
$ kopi global 21
Updating global JDK from 17.0.5 to 21.0.1
Set global JDK version to 21.0.1
This will be the default for new shells and projects without local versions
```

### Error Scenarios

#### Version Not Installed (Auto-Install Enabled)
```bash
$ kopi global 19
JDK 19 is not installed. Would you like to install it now? [Y/n] y
[kopi] Installing JDK 19...
[=====================================>] 100%
Successfully installed 19.0.2
Set global JDK version to 19.0.2
This will be the default for new shells and projects without local versions
```

#### Version Not Installed (User Declines)
```bash
$ kopi global 19
JDK 19 is not installed. Would you like to install it now? [Y/n] n
Error: JDK version '19' is not installed
Hint: Run 'kopi install 19' to install this version
```

#### Version Not Installed (Auto-Install Disabled)
```bash
$ kopi global 19
Error: JDK version '19' is not installed
Hint: Run 'kopi install 19' to install this version
Available installed versions:
  - 11.0.2
  - 17.0.5
  - 21.0.1
```

#### Permission Issues
```bash
$ kopi global 17
Error: Permission denied: cannot write to ~/.kopi/version
Hint: Check permissions for ~/.kopi directory
```

## Integration Points

### Shell Initialization

No shell configuration needed - shims automatically use global version:

```bash
# No need to add to .bashrc/.zshrc
# Shims handle version resolution automatically
```

### System Services

System services use the global version by default:

```bash
# Systemd service example
[Service]
ExecStart=/usr/bin/java -jar myapp.jar
# Will use Kopi global version via shims
```

### IDE Configuration

IDEs can detect the global Kopi version:
- Read from ~/.kopi/version
- Use ~/.kopi/default symlink
- Fallback for projects without local version

## Advanced Features

### Version Validation

```rust
fn validate_global_version(version: &str) -> Result<()> {
    // Check if version is installed
    if !is_version_installed(version)? {
        // List available versions
        let installed = list_installed_versions()?;
        
        if installed.is_empty() {
            return Err(KopiError::NoVersionsInstalled);
        }
        
        eprintln!("Available versions:");
        for v in installed {
            eprintln!("  - {}", v);
        }
        
        return Err(KopiError::VersionNotInstalled(version.to_string()));
    }
    
    Ok(())
}
```

### Migration from System JDK

Helper to migrate from system-installed JDK:

```rust
pub fn migrate_from_system() -> Result<()> {
    // Detect system JDK
    if let Some(system_java) = which::which("java").ok() {
        // Check if it's not already a Kopi shim
        if !is_kopi_shim(&system_java) {
            let version = detect_system_jdk_version()?;
            eprintln!("Detected system JDK: {}", version);
            eprintln!("Run 'kopi global {}' to use Kopi-managed version", version);
        }
    }
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests
- Version file creation and updates
- Directory permission handling
- Symlink creation on different platforms
- Version validation logic

### Integration Tests
- Shim fallback to global version
- System service integration
- Concurrent access to version file
- Platform-specific symlink behavior

### Test Scenarios
1. Set global version for first time
2. Update existing global version
3. Handle missing ~/.kopi directory
4. Permission denied scenarios
5. Invalid version handling

## Security Considerations

- Validate version strings before file operations
- Check directory ownership before writing
- Use atomic file writes to prevent corruption
- Restrict symlink targets to Kopi-managed JDKs

## Platform-Specific Behavior

### Unix/Linux/macOS
- Symlinks created with standard permissions
- Version file with 0644 permissions
- Directory with 0755 permissions

### Windows
- Directory junctions for symlinks
- Requires appropriate permissions for symlink creation
- Falls back to file copy if symlinks unavailable

## Future Enhancements

1. **Version Policies**: Enforce minimum version requirements
2. **Auto-update**: Optionally update to latest patch version
3. **Profiles**: Support for multiple named global configurations
4. **System Integration**: OS package manager integration
5. **Backup/Restore**: Save and restore global configurations