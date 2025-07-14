//! Platform-specific process execution.

use crate::error::{KopiError, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Execute a command, replacing the current process on Unix
#[cfg(unix)]
pub fn exec_replace(program: &Path, args: Vec<OsString>) -> std::io::Error {
    use std::os::unix::process::CommandExt;

    // exec() only returns on error
    Command::new(program).args(args).exec()
}

/// Execute a command on Windows (cannot replace process)
#[cfg(windows)]
pub fn exec_replace(program: &Path, args: Vec<OsString>) -> std::io::Error {
    use std::process::Stdio;

    match Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => {
            std::process::exit(status.code().unwrap_or(1));
        }
        Err(e) => e,
    }
}

/// Launch a shell with environment variable set on Unix
#[cfg(unix)]
pub fn launch_shell_with_env(shell_path: &PathBuf, env_name: &str, env_value: &str) -> Result<()> {
    use std::os::unix::process::CommandExt;

    // Build command with environment variable
    // Parent process environment is inherited by default
    let err = Command::new(shell_path)
        .env(env_name, env_value)
        .exec();

    // exec only returns on error
    Err(KopiError::SystemError(format!(
        "Failed to execute shell: {err}"
    )))
}

/// Launch a shell with environment variable set on Windows
#[cfg(windows)]
pub fn launch_shell_with_env(shell_path: &PathBuf, env_name: &str, env_value: &str) -> Result<()> {
    use std::process::Stdio;

    // On Windows, we can't replace the process, so spawn and wait
    let status = Command::new(shell_path)
        .env(env_name, env_value)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| KopiError::SystemError(format!("Failed to spawn shell: {}", e)))?;

    // Exit with the same code as the shell
    std::process::exit(status.code().unwrap_or(1));
}
