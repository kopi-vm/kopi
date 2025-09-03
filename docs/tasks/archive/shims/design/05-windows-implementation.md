# Windows Implementation

## Platform-Specific Features

### Process Spawning

Windows doesn't support exec(), so we spawn a new process and wait:

```rust
#[cfg(windows)]
fn main() {
    let tool_name = get_tool_name_windows();
    let tool_path = resolve_tool_path(&tool_name).unwrap();

    // Windows doesn't have exec(), so we spawn and wait
    let status = Command::new(&tool_path)
        .args(env::args_os().skip(1))
        .status()
        .unwrap_or_else(|e| {
            eprintln!("kopi: failed to execute {}: {}", tool_path.display(), e);
            std::process::exit(126);
        });

    std::process::exit(status.code().unwrap_or(1));
}
```

### Individual .exe Files

Windows requires individual .exe files for each tool:

```rust
#[cfg(windows)]
fn install_shim_binary(shim_dir: &Path) -> Result<()> {
    // On Windows, we need individual .exe files
    let source = env::current_exe()?
        .parent()
        .ok_or("Cannot find kopi installation directory")?
        .join("kopi-shim.exe");

    // Copy to shim directory as template
    let template = shim_dir.join("kopi-shim.exe");
    fs::copy(&source, &template)?;
    Ok(())
}

#[cfg(windows)]
fn create_shim_for_tool(shim_dir: &Path, tool_name: &str) -> Result<()> {
    // Windows: copy kopi-shim.exe to tool_name.exe
    let source = shim_dir.join("kopi-shim.exe");
    let dest = shim_dir.join(format!("{}.exe", tool_name));

    if dest.exists() {
        fs::remove_file(&dest)?;
    }
    fs::copy(&source, &dest)?;
    Ok(())
}
```

### Tool Name Detection

Windows-specific logic for handling .exe extensions:

```rust
#[cfg(windows)]
fn get_tool_name_windows() -> String {
    // Windows-specific logic to handle .exe extension
    let exe_path = env::current_exe().unwrap();
    exe_path.file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
```

### PATH Configuration

Windows PowerShell configuration:

```powershell
# PowerShell profile
$env:Path = "$env:USERPROFILE\.kopi\shims;$env:Path"

# Permanent PATH update
[Environment]::SetEnvironmentVariable(
    "Path",
    "$env:USERPROFILE\.kopi\shims;$([Environment]::GetEnvironmentVariable('Path', 'User'))",
    "User"
)
```

## Performance Characteristics

- **Process creation**: ~10-20ms (CreateProcess overhead)
- **File system**: NTFS generally slower than ext4/APFS
- **Total overhead**: 10-20ms typical

## Differences from Unix

| Aspect         | Unix                      | Windows                |
| -------------- | ------------------------- | ---------------------- |
| Process Model  | exec() replacement        | spawn + wait           |
| Shim Files     | Symlinks to single binary | Individual .exe copies |
| Performance    | 1-10ms                    | 10-20ms                |
| PATH Separator | :                         | ;                      |

## Next: [Performance Optimizations](./07-performance-optimizations.md)
