# Which Command Implementation Plan

## Overview
This document outlines the implementation plan for the `kopi which` command, which shows the installation path for JDK versions. The command provides a simple way to locate Java executables, other JDK tools, or the JDK home directory, supporting both current and specific JDK version queries with flexible output formats.

## Command Syntax
- `kopi which [<version>] [options]` - Show path to java executable or JDK home
  - `<version>` - Optional JDK version specification (e.g., `21`, `temurin@21.0.5+11`)
  - `--tool <name>` - Show path for specific JDK tool (default: java)
  - `--home` - Show JDK home directory instead of executable path
  - `--json` - Output in JSON format
- Alias: `w`

## Phase 1: Core Implementation

### Input Resources
- `/docs/tasks/archive/which/design.md` - Complete which command design
- `/src/version/resolver.rs` - Existing version resolution logic
- `/src/storage/jdk_repository.rs` - JDK installation management
- `/src/error/` - Error types and exit codes
- `/src/commands/mod.rs` - Command structure

### Deliverables
1. **Which Command Module** (`/src/commands/which.rs`)
   - Command handler implementation with clap derive:
     ```rust
     use clap::Args;
     use serde::Serialize;
     use crate::error::{KopiError, KopiResult};
     use crate::storage::JdkRepository;
     use crate::version::resolver::resolve_version;
     use crate::models::{JdkSpec, parse_jdk_spec};
     use std::path::PathBuf;

     #[derive(Args, Debug)]
     pub struct WhichCommand {
         /// JDK version specification (optional)
         version: Option<String>,

         /// Show path for specific JDK tool
         #[arg(long, default_value = "java")]
         tool: String,

         /// Show JDK home directory instead of executable path
         #[arg(long)]
         home: bool,

         /// Output in JSON format
         #[arg(long)]
         json: bool,
     }

     #[derive(Serialize)]
     struct WhichOutput {
         distribution: String,
         version: String,
         tool: String,
         tool_path: String,
         jdk_home: String,
         source: String,
     }
     ```

   - Version resolution logic:
     ```rust
     impl WhichCommand {
         pub fn execute(self) -> KopiResult<()> {
             let repo = JdkRepository::load()?;
             
             // Resolve JDK spec
             let (jdk_spec, source) = if let Some(version) = self.version {
                 // Parse specified version
                 let spec = parse_jdk_spec(&version)?;
                 (spec, "specified".to_string())
             } else {
                 // Use current version resolution
                 let (version_req, source) = resolve_version_with_source()?;
                 let version_req = version_req.ok_or(KopiError::NoLocalVersion {
                     searched_paths: vec![], // populated by resolver
                 })?;
                 let spec = JdkSpec::from_version_request(version_req)?;
                 (spec, format!("{:?}", source))
             };

             // Find installed JDK
             let installation = repo.find_installed_jdk(&jdk_spec)?
                 .ok_or_else(|| KopiError::JdkNotInstalled {
                     jdk_spec: jdk_spec.clone(),
                     version: jdk_spec.version().to_string(),
                     auto_install_enabled: false,
                 })?;

             // Determine output path
             let output_path = if self.home {
                 installation.path().to_path_buf()
             } else {
                 self.get_tool_path(&installation, &self.tool)?
             };

             // Output result
             if self.json {
                 self.output_json(&jdk_spec, &installation, &output_path, &source)?;
             } else {
                 println!("{}", output_path.display());
             }

             Ok(())
         }
     }
     ```

   - Tool path resolution:
     ```rust
     fn get_tool_path(&self, installation: &JdkInstallation, tool: &str) -> KopiResult<PathBuf> {
         let tool_name = if cfg!(windows) {
             format!("{}.exe", tool)
         } else {
             tool.to_string()
         };

         let tool_path = installation.path().join("bin").join(&tool_name);
         
         if !tool_path.exists() {
             return Err(KopiError::ToolNotFound {
                 tool: tool.to_string(),
                 jdk_path: installation.path().to_path_buf(),
             });
         }

         Ok(tool_path)
     }
     ```

   - Pattern matching for ambiguous versions:
     ```rust
     fn find_matching_jdk(&self, repo: &JdkRepository, spec: &JdkSpec) -> KopiResult<JdkInstallation> {
         let matches = repo.find_matching_jdks(spec)?;
         
         match matches.len() {
             0 => Err(KopiError::JdkNotInstalled {
                 jdk_spec: spec.clone(),
                 version: spec.version().to_string(),
                 auto_install_enabled: false,
             }),
             1 => Ok(matches.into_iter().next().unwrap()),
             _ => {
                 // Multiple matches - need disambiguation
                 let versions: Vec<String> = matches.iter()
                     .map(|jdk| format!("{}@{}", jdk.distribution(), jdk.version()))
                     .collect();
                 Err(KopiError::ValidationError(
                     format!("Multiple JDKs match version '{}'\n\nFound:\n  {}\n\nPlease specify the full version or distribution",
                         spec.version(),
                         versions.join("\n  ")
                     )
                 ))
             }
         }
     }
     ```

2. **Version Resolution Enhancement** (`/src/version/resolver.rs`)
   - Add `resolve_version_with_source()` if not already present:
     ```rust
     pub fn resolve_version_with_source() -> KopiResult<(Option<VersionRequest>, VersionSource)> {
         // Check KOPI_JAVA_VERSION environment variable
         if let Ok(version) = std::env::var("KOPI_JAVA_VERSION") {
             return Ok((Some(VersionRequest::parse(&version)?), VersionSource::Environment));
         }

         // Check project files
         if let Some((version, path)) = find_project_version_file()? {
             return Ok((Some(version), VersionSource::ProjectFile(path)));
         }

         // Check global configuration
         if let Some(version) = read_global_version()? {
             return Ok((Some(version), VersionSource::GlobalDefault));
         }

         Ok((None, VersionSource::None))
     }
     ```

3. **Error Type Enhancement** (`/src/error/mod.rs`)
   - Add `ToolNotFound` error variant if not present:
     ```rust
     #[derive(Error, Debug)]
     pub enum KopiError {
         // ... existing variants
         
         #[error("Tool '{tool}' not found in JDK installation at {jdk_path}")]
         ToolNotFound {
             tool: String,
             jdk_path: PathBuf,
         },
     }
     ```

4. **CLI Integration** (update `/src/main.rs`)
   - Add `Which` command to Commands enum:
     ```rust
     #[derive(Subcommand)]
     enum Commands {
         // ... existing commands
         
         /// Show installation path for a JDK version
         Which(which::WhichCommand),
         
         /// Show installation path for a JDK version (alias)
         #[command(visible_alias = "w")]
         W(which::WhichCommand),
     }
     ```

   - Add command routing:
     ```rust
     match cli.command {
         // ... existing matches
         Commands::Which(cmd) => cmd.execute()?,
         Commands::W(cmd) => cmd.execute()?,
     }
     ```

5. **Module Registration** (update `/src/commands/mod.rs`)
   ```rust
   pub mod which;
   ```

### Success Criteria
- Command correctly resolves current JDK when no version specified
- Specific version lookup works with pattern matching
- Tool path resolution works for various JDK tools
- `--home` option returns JDK directory instead of executable
- JSON output properly formatted
- Error messages clear and actionable
- Exit codes match error types from `src/error/exit_codes.rs`

## Phase 2: Testing and Polish

### Input Resources
- Phase 1 deliverables
- `/tests/common/` - Test utilities
- Existing test patterns from other commands

### Deliverables
1. **Unit Tests** (`/src/commands/which.rs` test module)
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use tempfile::TempDir;
       use crate::test::fixtures::create_test_jdk;

       #[test]
       fn test_which_current_version() {
           let temp_dir = TempDir::new().unwrap();
           let repo = create_test_repository(&temp_dir);
           
           // Set up environment
           std::env::set_var("KOPI_JAVA_VERSION", "temurin@21");
           
           let cmd = WhichCommand {
               version: None,
               tool: "java".to_string(),
               home: false,
               json: false,
           };
           
           // Should find current version
           assert!(cmd.execute().is_ok());
       }

       #[test]
       fn test_which_specific_version() {
           let temp_dir = TempDir::new().unwrap();
           let repo = create_test_repository(&temp_dir);
           
           // Install test JDK
           create_test_jdk(&repo, "temurin", "21.0.5+11");
           
           let cmd = WhichCommand {
               version: Some("temurin@21".to_string()),
               tool: "java".to_string(),
               home: false,
               json: false,
           };
           
           assert!(cmd.execute().is_ok());
       }

       #[test]
       fn test_which_tool_not_found() {
           let temp_dir = TempDir::new().unwrap();
           let repo = create_test_repository(&temp_dir);
           
           create_test_jdk(&repo, "temurin", "21.0.5+11");
           
           let cmd = WhichCommand {
               version: Some("temurin@21".to_string()),
               tool: "nonexistent-tool".to_string(),
               home: false,
               json: false,
           };
           
           match cmd.execute() {
               Err(KopiError::ToolNotFound { tool, .. }) => {
                   assert_eq!(tool, "nonexistent-tool");
               }
               _ => panic!("Expected ToolNotFound error"),
           }
       }

       #[test]
       fn test_which_home_option() {
           let temp_dir = TempDir::new().unwrap();
           let repo = create_test_repository(&temp_dir);
           
           let jdk = create_test_jdk(&repo, "temurin", "21.0.5+11");
           
           let cmd = WhichCommand {
               version: Some("temurin@21".to_string()),
               tool: "java".to_string(),
               home: true,
               json: false,
           };
           
           // Should return JDK home, not executable path
           let output = capture_stdout(|| cmd.execute());
           assert_eq!(output.trim(), jdk.path().display().to_string());
       }

       #[test]
       fn test_which_json_output() {
           let temp_dir = TempDir::new().unwrap();
           let repo = create_test_repository(&temp_dir);
           
           create_test_jdk(&repo, "temurin", "21.0.5+11");
           
           let cmd = WhichCommand {
               version: Some("temurin@21".to_string()),
               tool: "javac".to_string(),
               home: false,
               json: true,
           };
           
           let output = capture_stdout(|| cmd.execute());
           let json: serde_json::Value = serde_json::from_str(&output).unwrap();
           
           assert_eq!(json["tool"], "javac");
           assert_eq!(json["distribution"], "temurin");
           assert_eq!(json["version"], "21.0.5+11");
       }

       #[test]
       fn test_ambiguous_version() {
           let temp_dir = TempDir::new().unwrap();
           let repo = create_test_repository(&temp_dir);
           
           // Install multiple JDKs with same major version
           create_test_jdk(&repo, "temurin", "21.0.5+11");
           create_test_jdk(&repo, "corretto", "21.0.7.6.1");
           
           let cmd = WhichCommand {
               version: Some("21".to_string()),
               tool: "java".to_string(),
               home: false,
               json: false,
           };
           
           match cmd.execute() {
               Err(KopiError::ValidationError(msg)) => {
                   assert!(msg.contains("Multiple JDKs match"));
                   assert!(msg.contains("temurin@21"));
                   assert!(msg.contains("corretto@21"));
               }
               _ => panic!("Expected ValidationError for ambiguous version"),
           }
       }
   }
   ```

2. **Integration Tests** (`/tests/which.rs`)
   ```rust
   #[path = "common/mod.rs"]
   mod common;

   use common::{TestHomeGuard, run_kopi_command};
   use predicates::prelude::*;

   #[test]
   fn test_which_command_basic() {
       let _guard = TestHomeGuard::new();
       
       // Install a JDK first
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       // Test basic which
       run_kopi_command(&["which", "temurin@21"])
           .assert()
           .success()
           .stdout(predicate::str::contains("/bin/java"));
   }

   #[test]
   fn test_which_current_project() {
       let _guard = TestHomeGuard::new();
       
       // Install and set local version
       run_kopi_command(&["install", "temurin@17"])
           .assert()
           .success();
       
       run_kopi_command(&["local", "temurin@17"])
           .assert()
           .success();
       
       // Which without version should find project version
       run_kopi_command(&["which"])
           .assert()
           .success()
           .stdout(predicate::str::contains("temurin-17"));
   }

   #[test]
   fn test_which_tools() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       // Test various tools
       for tool in &["java", "javac", "jar", "jshell"] {
           run_kopi_command(&["which", "--tool", tool, "temurin@21"])
               .assert()
               .success()
               .stdout(predicate::str::contains(tool));
       }
   }

   #[test]
   fn test_which_home_option() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       // Home option should not include /bin/java
       run_kopi_command(&["which", "--home", "temurin@21"])
           .assert()
           .success()
           .stdout(predicate::str::contains("temurin-21").and(
               predicate::str::contains("/bin/java").not()
           ));
   }

   #[test]
   fn test_which_json_format() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       let output = run_kopi_command(&["which", "--json", "temurin@21"])
           .assert()
           .success()
           .get_output()
           .stdout
           .clone();
       
       let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
       assert_eq!(json["distribution"], "temurin");
       assert_eq!(json["tool"], "java");
       assert!(json["tool_path"].as_str().unwrap().contains("java"));
   }

   #[test]
   fn test_which_not_installed() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["which", "temurin@22"])
           .assert()
           .failure()
           .code(4) // JdkNotInstalled
           .stderr(predicate::str::contains("not installed"));
   }

   #[test]
   fn test_which_tool_not_found() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       run_kopi_command(&["which", "--tool", "nonexistent", "temurin@21"])
           .assert()
           .failure()
           .code(5) // ToolNotFound
           .stderr(predicate::str::contains("Tool 'nonexistent' not found"));
   }
   ```

3. **Platform-Specific Tests** (`/tests/which_platform.rs`)
   ```rust
   #[cfg(windows)]
   #[test]
   fn test_which_windows_exe() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       // Windows should include .exe
       run_kopi_command(&["which", "temurin@21"])
           .assert()
           .success()
           .stdout(predicate::str::contains("java.exe"));
   }

   #[cfg(unix)]
   #[test]
   fn test_which_unix_no_exe() {
       let _guard = TestHomeGuard::new();
       
       run_kopi_command(&["install", "temurin@21"])
           .assert()
           .success();
       
       // Unix should not include .exe
       run_kopi_command(&["which", "temurin@21"])
           .assert()
           .success()
           .stdout(predicate::str::contains("/java").and(
               predicate::str::contains(".exe").not()
           ));
   }
   ```

4. **Benchmark Tests** (`/benches/which_bench.rs`)
   ```rust
   use criterion::{black_box, criterion_group, criterion_main, Criterion};
   use kopi::commands::which::WhichCommand;

   fn bench_which_current(c: &mut Criterion) {
       c.bench_function("which_current", |b| {
           b.iter(|| {
               let cmd = WhichCommand {
                   version: None,
                   tool: black_box("java".to_string()),
                   home: false,
                   json: false,
               };
               let _ = cmd.execute();
           });
       });
   }

   fn bench_which_specific(c: &mut Criterion) {
       c.bench_function("which_specific", |b| {
           b.iter(|| {
               let cmd = WhichCommand {
                   version: Some(black_box("temurin@21".to_string())),
                   tool: black_box("java".to_string()),
                   home: false,
                   json: false,
               };
               let _ = cmd.execute();
           });
       });
   }

   criterion_group!(benches, bench_which_current, bench_which_specific);
   criterion_main!(benches);
   ```

### Success Criteria
- All unit tests pass with good coverage
- Integration tests verify end-to-end functionality
- Platform-specific behavior tested on Windows and Unix
- Performance benchmarks show < 20ms typical execution time
- Error scenarios properly tested
- JSON output validates against expected schema

## Implementation Guidelines

### Development Process
1. Start with `/clear` command to reset context
2. Load this plan.md and design.md
3. Implement core functionality first
4. Add tests incrementally
5. Run quality checks after each module:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo check`
   - `cargo test --lib --quiet`
   - `cargo test --test which` (integration tests)

### Code Quality Standards
- Use existing error types and patterns
- Follow Rust idioms and project conventions
- Document public APIs
- Handle all error cases explicitly
- Minimize allocations in hot paths

### Testing Strategy
- Unit tests use mocks for JdkRepository
- Integration tests use real filesystem
- Test both success and error paths
- Verify exit codes match specifications
- Test cross-platform behavior

## Design Principles

### Simplicity
- Command does one thing well: show paths
- Minimal output by default (just the path)
- No additional information unless requested

### Flexibility
- Support any JDK tool via `--tool`
- Provide JDK home with `--home`
- JSON output for scripting

### Consistency
- Reuse existing version resolution
- Match error handling patterns
- Follow project conventions

### Performance
- Fast execution (< 20ms target)
- Minimal file I/O
- Efficient path construction

## Success Metrics
- Command executes in < 20ms for typical use
- Exit codes correctly indicate error types
- Works reliably across platforms
- Integrates seamlessly with existing commands
- Clear, actionable error messages

## Dependencies Required
No new dependencies needed. Uses existing:
- **clap**: Command-line parsing
- **serde/serde_json**: JSON output
- **Standard library**: Path manipulation

## Next Steps
Begin with Phase 1, implementing the core WhichCommand module and integrating it with the CLI. Focus on getting basic functionality working before adding all features.