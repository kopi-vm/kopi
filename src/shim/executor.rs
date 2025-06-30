//! Process execution for shims.
//!
//! This module handles the platform-specific execution of Java tools,
//! using exec() on Unix systems and CreateProcess on Windows.

use crate::error::{KopiError, Result};
use std::ffi::OsString;
use std::path::PathBuf;

/// Platform-specific executor for launching Java tools.
pub struct ShimExecutor;

impl ShimExecutor {
    /// Execute the Java tool, replacing the current process.
    ///
    /// On Unix systems, this uses exec() to replace the current process.
    /// On Windows, this uses CreateProcess and waits for completion.
    ///
    /// The executed process inherits:
    /// - All environment variables
    /// - Standard input/output/error streams
    /// - Working directory
    #[cfg(unix)]
    pub fn exec(tool_path: PathBuf, args: Vec<OsString>) -> Result<()> {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        // Build command - exec() inherits everything automatically
        let err = Command::new(&tool_path).args(args).exec();

        // exec() only returns on error
        Err(KopiError::SystemError(format!(
            "Failed to execute {:?}: {}",
            tool_path, err
        )))
    }

    /// Execute the Java tool on Windows.
    #[cfg(windows)]
    pub fn exec(tool_path: PathBuf, args: Vec<OsString>) -> Result<()> {
        use std::process::{Command, Stdio};

        let mut command = Command::new(&tool_path);
        command
            .args(args)
            // Inherit stdio to ensure output goes to the terminal
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = command.status().map_err(|e| {
            KopiError::SystemError(format!("Failed to execute {:?}: {}", tool_path, e))
        })?;

        // Exit with the same code as the child process
        std::process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_construction() {
        // Just verify we can construct the executor
        // Actual execution tests need to be in integration tests
        let _executor = ShimExecutor;
    }

    // Platform-specific execution tests would go in integration tests
    // as they need to actually spawn processes
}
