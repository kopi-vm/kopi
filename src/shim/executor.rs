use crate::error::{KopiError, Result};
use std::ffi::OsString;
use std::path::PathBuf;

pub struct ShimExecutor;

impl ShimExecutor {
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
