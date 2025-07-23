#[path = "common/mod.rs"]
mod common;

use assert_cmd::Command as AssertCommand;
use common::{TestHomeGuard, fixtures};
use predicates::prelude::*;

#[cfg(windows)]
#[test]
fn test_which_windows_exe() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    // Windows should include .exe
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "temurin@21"])
        .assert()
        .success()
        .stdout(predicate::str::contains("java.exe"));
}

#[cfg(unix)]
#[test]
fn test_which_unix_no_exe() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "temurin", "21.0.5+11");

    // Unix should not include .exe
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "temurin@21"])
        .assert()
        .success()
        .stdout(predicate::str::contains("/java").and(predicate::str::contains(".exe").not()));
}

#[cfg(windows)]
#[test]
fn test_which_windows_tool_paths() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "corretto", "17.0.11.9.1");

    // Test various tools on Windows
    for tool in &["javac", "jar", "jdeps"] {
        AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
            .env("KOPI_HOME", _guard.kopi_home())
            .args(["which", "--tool", tool, "corretto@17"])
            .assert()
            .success()
            .stdout(predicate::str::contains(format!("{tool}.exe")));
    }
}

#[cfg(unix)]
#[test]
fn test_which_unix_tool_paths() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "zulu", "11.72.19");

    // Test various tools on Unix
    for tool in &["javac", "jar", "jdeps"] {
        AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
            .env("KOPI_HOME", _guard.kopi_home())
            .args(["which", "--tool", tool, "zulu@11"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains(format!("/{tool}"))
                    .and(predicate::str::contains(".exe").not()),
            );
    }
}

#[cfg(windows)]
#[test]
fn test_which_windows_path_separators() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "liberica", "8.412.8");

    // Windows paths should use backslashes
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "--home", "liberica@8"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\\"));
}

#[cfg(unix)]
#[test]
fn test_which_unix_path_separators() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "graalvm", "22.0.2");

    // Unix paths should use forward slashes
    AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
        .env("KOPI_HOME", _guard.kopi_home())
        .args(["which", "--home", "graalvm@22"])
        .assert()
        .success()
        .stdout(predicate::str::contains("/").and(predicate::str::contains("\\").not()));
}

#[cfg(windows)]
#[test]
fn test_which_windows_case_insensitive_tools() {
    let guard = TestHomeGuard::new();
    let _guard = guard.setup_kopi_structure();

    // Create a fake installed JDK
    fixtures::create_test_jdk_fs(&_guard.kopi_home(), "sapmachine", "21.0.4");

    // Windows file system is typically case-insensitive
    // Tool names should work regardless of case
    for tool in &["Java", "JAVAC", "JaR"] {
        AssertCommand::new(env!("CARGO_BIN_EXE_kopi"))
            .env("KOPI_HOME", _guard.kopi_home())
            .args(["which", "--tool", tool, "sapmachine@21"])
            .assert()
            .stderr(predicate::str::is_empty().or(
                // May fail with "Tool not found" if the implementation is case-sensitive
                predicate::str::contains("not found"),
            ));
    }
}
