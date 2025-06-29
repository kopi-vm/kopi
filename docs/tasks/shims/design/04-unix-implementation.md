# Unix Implementation

## Platform-Specific Features

### Process Replacement with exec()

On Unix systems, we use `exec()` to replace the shim process entirely with the target JDK tool. This eliminates process chains and ensures minimal overhead.

```rust
use std::os::unix::process::CommandExt;

// Replace current process with actual tool
let err = Command::new(&tool_path)
    .args(env::args_os().skip(1))
    .exec();

// Only reached if exec fails
eprintln!("kopi: failed to execute {}: {}", tool_path.display(), err);
std::process::exit(126);
```

### Symlink-Based Shims

Unix systems use symlinks pointing to a single `kopi-shim` binary:

```rust
#[cfg(unix)]
fn install_shim_binary(shim_dir: &Path) -> Result<()> {
    // Copy the pre-compiled kopi-shim binary from the Kopi installation
    let source = env::current_exe()?
        .parent()
        .ok_or("Cannot find kopi installation directory")?
        .join("kopi-shim");
    
    let dest = shim_dir.join("kopi-shim");
    fs::copy(&source, &dest)?;
    
    // Make it executable
    let mut perms = fs::metadata(&dest)?.permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    fs::set_permissions(&dest, perms)?;
    
    Ok(())
}

#[cfg(unix)]
fn create_shim_for_tool(shim_dir: &Path, tool_name: &str) -> Result<()> {
    let shim_path = shim_dir.join(tool_name);
    
    // Create symlink to kopi-shim (relative path for portability)
    if shim_path.exists() {
        fs::remove_file(&shim_path)?;
    }
    std::os::unix::fs::symlink("kopi-shim", &shim_path)?;
    Ok(())
}
```

### Permission Handling

Unix systems require proper permission management:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    if permissions.mode() & 0o111 == 0 {
        return Err("Tool is not executable".into());
    }
}
```

### Shell Configuration

Unix shells require PATH configuration:

```bash
# .bashrc / .zshrc
export PATH="$HOME/.kopi/shims:$PATH"

# Fish shell
set -x PATH $HOME/.kopi/shims $PATH
```

## Performance Characteristics

- **Process overhead**: Eliminated through exec()
- **Symlink resolution**: < 1ms
- **Total overhead**: 1-10ms typical

## Next: [Windows Implementation](./05-windows-implementation.md)