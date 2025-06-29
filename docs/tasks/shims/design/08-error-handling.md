# Error Handling and Automatic Installation

## Error Types

```rust
enum ShimError {
    NoVersionFound,
    VersionNotInstalled(String),
    ToolNotFound(String, String),
    ExecutionFailed(std::io::Error),
    InstallationFailed(String),
}

impl Display for ShimError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ShimError::NoVersionFound => {
                write!(f, "No Java version specified. Create a .kopi-version file or run 'kopi use <version>'")
            }
            ShimError::VersionNotInstalled(v) => {
                write!(f, "Java {} is not installed. Installing automatically...", v)
            }
            ShimError::ToolNotFound(tool, version) => {
                write!(f, "Tool '{}' not found in Java {}. The JDK installation may be corrupted.", tool, version)
            }
            ShimError::ExecutionFailed(e) => {
                write!(f, "Failed to execute tool: {}", e)
            }
            ShimError::InstallationFailed(msg) => {
                write!(f, "Failed to install JDK: {}", msg)
            }
        }
    }
}
```

## Automatic Installation Flow

When a required JDK version is not found, the shim automatically installs it:

```rust
fn main() {
    let tool_name = get_tool_name();
    let version = match resolve_version(&env::current_dir().unwrap()) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("kopi: No Java version specified");
            std::process::exit(1);
        }
    };
    
    // Check if JDK is installed
    let jdk_path = get_jdk_path(&version);
    if !jdk_path.exists() {
        // Automatic installation
        if let Err(e) = install_jdk_from_shim(&version) {
            eprintln!("kopi: Failed to install Java {}: {}", version, e);
            std::process::exit(1);
        }
    }
    
    // Now execute the tool
    let tool_path = jdk_path.join("bin").join(&tool_name);
    execute_tool(&tool_path);
}

fn install_jdk_from_shim(version: &Version) -> Result<()> {
    eprintln!("kopi: Java {} is not installed. Installing automatically...", version);
    
    // Show progress indicator
    let spinner = ProgressBar::new_spinner();
    spinner.set_message(format!("Installing Java {}...", version));
    spinner.enable_steady_tick(100);
    
    // Call kopi install as a subprocess
    let output = Command::new(find_kopi_binary()?)
        .arg("install")
        .arg(version.to_string())
        .arg("--quiet")  // Suppress normal output
        .output()?;
    
    spinner.finish_and_clear();
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Installation failed: {}", stderr);
    }
    
    eprintln!("kopi: Successfully installed Java {}", version);
    Ok(())
}

fn find_kopi_binary() -> Result<PathBuf> {
    // 1. Check if kopi is in the same directory as the shim
    let shim_dir = env::current_exe()?.parent().unwrap().to_path_buf();
    let kopi_path = shim_dir.join("kopi");
    
    #[cfg(windows)]
    let kopi_path = kopi_path.with_extension("exe");
    
    if kopi_path.exists() {
        return Ok(kopi_path);
    }
    
    // 2. Check ~/.kopi/bin/kopi
    if let Some(home) = dirs::home_dir() {
        let kopi_home = home.join(".kopi/bin/kopi");
        
        #[cfg(windows)]
        let kopi_home = kopi_home.with_extension("exe");
        
        if kopi_home.exists() {
            return Ok(kopi_home);
        }
    }
    
    // 3. Fall back to PATH
    which::which("kopi").map_err(|_| anyhow!("Cannot find kopi binary"))
}
```

## User Experience Considerations

### First-time Experience
```bash
$ cd my-project
$ cat .kopi-version
17.0.9
$ java -version
kopi: Java 17.0.9 is not installed. Installing automatically...
Installing Java 17.0.9... ✓
kopi: Successfully installed Java 17.0.9
openjdk version "17.0.9" 2023-10-17
OpenJDK Runtime Environment Temurin-17.0.9+9 (build 17.0.9+9)
```

### Configuration Options
```toml
# ~/.kopi/config.toml
[shims]
auto_install = true  # Default: true
install_timeout = 300  # Seconds, default: 300 (5 minutes)

# Prompt before installing
auto_install_prompt = true  # Default: false
```

### Interactive Prompt Option
```rust
fn should_install_jdk(version: &Version) -> Result<bool> {
    let config = KopiConfig::load()?;
    
    if !config.shims.auto_install {
        return Ok(false);
    }
    
    if config.shims.auto_install_prompt {
        // Interactive prompt
        eprint!("kopi: Java {} is not installed. Install it now? [Y/n] ", version);
        std::io::stdout().flush()?;
        
        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;
        
        match response.trim().to_lowercase().as_str() {
            "" | "y" | "yes" => Ok(true),
            _ => Ok(false),
        }
    } else {
        Ok(true)
    }
}
```

## Performance Considerations for Auto-Installation

1. **First-run Penalty**: Installation only happens once per version
2. **Parallel Installation**: Multiple shims waiting for the same version coordinate via lock file
3. **Network Timeout**: Configurable timeout prevents hanging on slow networks
4. **Fallback Behavior**: If installation fails, subsequent attempts are skipped for a period

### Coordination Between Multiple Shim Processes

```rust
// Coordination between multiple shim processes
fn install_jdk_with_coordination(version: &Version) -> Result<()> {
    let lock_file = get_kopi_home()?.join(".locks").join(format!("{}.lock", version));
    
    // Try to acquire exclusive lock
    let lock = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_file)?;
    
    match lock.try_lock_exclusive() {
        Ok(_) => {
            // We got the lock, proceed with installation
            install_jdk_from_shim(version)?;
            Ok(())
        }
        Err(_) => {
            // Another process is installing, wait for it
            eprintln!("kopi: Another process is installing Java {}. Waiting...", version);
            
            // Wait with timeout
            let start = Instant::now();
            let timeout = Duration::from_secs(300); // 5 minutes
            
            loop {
                if get_jdk_path(version).exists() {
                    eprintln!("kopi: Java {} is now available", version);
                    return Ok(());
                }
                
                if start.elapsed() > timeout {
                    bail!("Timeout waiting for Java {} installation", version);
                }
                
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
```

## Next: [Distribution-Specific Tools](./09-distribution-specific-tools.md)