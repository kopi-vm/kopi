// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Platform-specific process execution.

use crate::error::{KopiError, Result};
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Metadata about a process that is interacting with a JDK installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessInfo {
    /// Operating system process identifier.
    pub pid: u32,
    /// Path to the executable that owns the process.
    pub exe_path: PathBuf,
    /// Collection of open handle paths rooted inside the monitored JDK directory.
    pub handles: Vec<PathBuf>,
}

/// Enumerate processes that hold open handles beneath the provided directory.
pub fn processes_using_path(target: &Path) -> Result<Vec<ProcessInfo>> {
    let canonical_target = normalize_target(target)?;
    platform_processes_using_path(&canonical_target)
}

fn normalize_target(target: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(target).map_err(|err| match err.kind() {
        ErrorKind::NotFound => KopiError::DirectoryNotFound(target.display().to_string()),
        ErrorKind::PermissionDenied => {
            KopiError::PermissionDenied(format!("Unable to access {}: {err}", target.display()))
        }
        _ => KopiError::SystemError(format!(
            "Failed to canonicalize {}: {err}",
            target.display()
        )),
    })?;

    let metadata = fs::metadata(&canonical).map_err(|err| match err.kind() {
        ErrorKind::PermissionDenied => {
            KopiError::PermissionDenied(format!("Unable to inspect {}: {err}", canonical.display()))
        }
        _ => KopiError::SystemError(format!("Failed to inspect {}: {err}", canonical.display())),
    })?;

    if !metadata.is_dir() {
        return Err(KopiError::ValidationError(format!(
            "Process detection target must be a directory: {}",
            canonical.display()
        )));
    }

    Ok(canonical)
}

#[cfg(target_os = "linux")]
fn platform_processes_using_path(_target: &Path) -> Result<Vec<ProcessInfo>> {
    Err(KopiError::NotImplemented(
        "Process activity detection for Linux is not implemented yet".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn platform_processes_using_path(_target: &Path) -> Result<Vec<ProcessInfo>> {
    Err(KopiError::NotImplemented(
        "Process activity detection for macOS is not implemented yet".to_string(),
    ))
}

#[cfg(windows)]
fn platform_processes_using_path(_target: &Path) -> Result<Vec<ProcessInfo>> {
    Err(KopiError::NotImplemented(
        "Process activity detection for Windows is not implemented yet".to_string(),
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_processes_using_path(_target: &Path) -> Result<Vec<ProcessInfo>> {
    Err(KopiError::NotImplemented(
        "Process activity detection is not supported on this platform".to_string(),
    ))
}

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
    let err = Command::new(shell_path).env(env_name, env_value).exec();

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
        .map_err(|e| KopiError::SystemError(format!("Failed to spawn shell: {e}")))?;

    // Exit with the same code as the shell
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::KopiError;
    use std::fs;

    #[test]
    fn normalize_target_returns_canonical_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let nested = temp_dir.path().join("nested");
        fs::create_dir(&nested).expect("create nested dir");

        let normalized = normalize_target(&nested).expect("normalize succeeds");
        let expected = fs::canonicalize(&nested).expect("canonical path");

        assert_eq!(normalized, expected);
    }

    #[test]
    fn normalize_target_rejects_missing_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let missing = temp_dir.path().join("missing");

        let err = normalize_target(&missing).expect_err("expected error for missing path");
        match err {
            KopiError::DirectoryNotFound(message) => {
                assert!(message.contains("missing"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn normalize_target_rejects_file_path() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, b"data").expect("write test file");

        let err = normalize_target(&file_path).expect_err("expected validation error");
        match err {
            KopiError::ValidationError(message) => {
                assert!(message.contains("must be a directory"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn processes_using_path_returns_not_implemented_for_current_platform() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let err = processes_using_path(temp_dir.path()).expect_err("expected placeholder error");
        assert!(matches!(err, KopiError::NotImplemented(_)));
    }
}
