//! Platform-specific process execution.

use std::ffi::OsString;
use std::path::Path;

/// Execute a command, replacing the current process on Unix
#[cfg(unix)]
pub fn exec_replace(program: &Path, args: Vec<OsString>) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    // exec() only returns on error
    Command::new(program).args(args).exec()
}

/// Execute a command on Windows (cannot replace process)
#[cfg(windows)]
pub fn exec_replace(program: &Path, args: Vec<OsString>) -> std::io::Error {
    use std::process::{Command, Stdio};

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
