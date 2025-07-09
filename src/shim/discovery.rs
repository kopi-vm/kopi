use std::path::Path;

use log::debug;

use crate::error::Result;
use crate::platform::file_ops::is_executable;

use super::tools::ToolRegistry;

/// Discovers available JDK tools in an installed JDK.
///
/// Scans the bin directory of the JDK installation and identifies
/// executable files that match known JDK tools from the ToolRegistry.
pub fn discover_jdk_tools(jdk_path: &Path) -> Result<Vec<String>> {
    let bin_dir = jdk_path.join("bin");

    if !bin_dir.exists() {
        return Ok(Vec::new());
    }

    let mut discovered_tools = Vec::new();
    let registry = ToolRegistry::new();

    // Read all entries in the bin directory
    let entries = std::fs::read_dir(&bin_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip if not a file
        if !path.is_file() {
            continue;
        }

        // Get the file name without extension
        let file_name = if cfg!(windows) {
            // On Windows, strip .exe extension if present
            match path.file_stem() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            }
        } else {
            // On Unix, use the full file name
            match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            }
        };

        // Check if this is an executable
        if !is_executable(&path)? {
            debug!("Skipping non-executable file: {}", path.display());
            continue;
        }

        // Check if this tool is in our registry
        if let Some(tool_info) = registry.get_tool(&file_name) {
            debug!(
                "Discovered JDK tool: {} ({})",
                tool_info.name, tool_info.description
            );
            discovered_tools.push(file_name);
        } else {
            debug!("Unknown executable in JDK bin: {file_name}");
        }
    }

    discovered_tools.sort();
    discovered_tools.dedup();

    Ok(discovered_tools)
}

/// Discovers distribution-specific tools that may not be in the standard JDK.
///
/// Some distributions include additional tools:
/// - GraalVM: native-image
/// - IBM Semeru/OpenJ9: jdmpview, jitserver, jpackcore, traceformat
pub fn discover_distribution_tools(
    jdk_path: &Path,
    distribution: Option<&str>,
) -> Result<Vec<String>> {
    let mut extra_tools = Vec::new();

    if let Some(dist) = distribution {
        match dist.to_lowercase().as_str() {
            "graalvm" | "graal" => {
                // Check for GraalVM-specific tools
                let bin_dir = jdk_path.join("bin");

                for tool in &[
                    "native-image",
                    "native-image-configure",
                    "native-image-inspect",
                ] {
                    let tool_name = if cfg!(windows) {
                        format!("{tool}.exe")
                    } else {
                        tool.to_string()
                    };
                    let tool_path = bin_dir.join(&tool_name);
                    if tool_path.exists() && is_executable(&tool_path)? {
                        debug!("Discovered GraalVM tool: {tool}");
                        extra_tools.push(tool.to_string());
                    }
                }
            }
            "semeru" | "openj9" => {
                // Check for IBM Semeru/OpenJ9-specific tools
                let bin_dir = jdk_path.join("bin");

                for tool in &["jdmpview", "jitserver", "jpackcore", "traceformat"] {
                    let tool_name = if cfg!(windows) {
                        format!("{tool}.exe")
                    } else {
                        tool.to_string()
                    };
                    let tool_path = bin_dir.join(&tool_name);
                    if tool_path.exists() && is_executable(&tool_path)? {
                        debug!("Discovered Semeru/OpenJ9 tool: {tool}");
                        extra_tools.push(tool.to_string());
                    }
                }
            }
            _ => {
                // No special handling for other distributions yet
            }
        }
    }

    Ok(extra_tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_jdk_tools_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();

        let tools = discover_jdk_tools(jdk_path).unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_discover_jdk_tools_no_bin_dir() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();

        // Create a JDK directory without a bin subdirectory
        fs::create_dir(jdk_path.join("lib")).unwrap();

        let tools = discover_jdk_tools(jdk_path).unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_discover_jdk_tools_with_standard_tools() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();
        let bin_dir = jdk_path.join("bin");
        fs::create_dir(&bin_dir).unwrap();

        // Create dummy JDK tools
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            for tool in &["java", "javac", "jar", "jdb"] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();
                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();
            }

            // Create a non-executable file that should be ignored
            let non_exec_path = bin_dir.join("README");
            fs::write(&non_exec_path, "This is not a tool").unwrap();

            // Create an unknown executable that should be ignored
            let unknown_path = bin_dir.join("unknown-tool");
            fs::write(&unknown_path, "#!/bin/sh\necho test").unwrap();
            fs::set_permissions(&unknown_path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        #[cfg(windows)]
        {
            for tool in &["java.exe", "javac.exe", "jar.exe", "jdb.exe"] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "test").unwrap();
            }

            // Create files that should be ignored
            fs::write(bin_dir.join("README.txt"), "This is not a tool").unwrap();
            fs::write(bin_dir.join("unknown-tool.exe"), "test").unwrap();
        }

        let tools = discover_jdk_tools(jdk_path).unwrap();

        // Should only find the known JDK tools
        assert_eq!(tools.len(), 4);
        assert!(tools.contains(&"java".to_string()));
        assert!(tools.contains(&"javac".to_string()));
        assert!(tools.contains(&"jar".to_string()));
        assert!(tools.contains(&"jdb".to_string()));

        // Should not include unknown tools or non-executables
        assert!(!tools.contains(&"README".to_string()));
        assert!(!tools.contains(&"unknown-tool".to_string()));
    }

    #[test]
    fn test_discover_distribution_tools_graalvm() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();
        let bin_dir = jdk_path.join("bin");
        fs::create_dir(&bin_dir).unwrap();

        // Create dummy GraalVM tools
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            for tool in &[
                "native-image",
                "native-image-configure",
                "native-image-inspect",
            ] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();
                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        #[cfg(windows)]
        {
            for tool in &[
                "native-image.exe",
                "native-image-configure.exe",
                "native-image-inspect.exe",
            ] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "test").unwrap();
            }
        }

        let tools = discover_distribution_tools(jdk_path, Some("graalvm")).unwrap();

        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"native-image".to_string()));
        assert!(tools.contains(&"native-image-configure".to_string()));
        assert!(tools.contains(&"native-image-inspect".to_string()));
    }

    #[test]
    fn test_discover_distribution_tools_non_graalvm() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();

        // For non-GraalVM/non-Semeru distributions, should return empty
        let tools = discover_distribution_tools(jdk_path, Some("temurin")).unwrap();
        assert!(tools.is_empty());

        let tools = discover_distribution_tools(jdk_path, Some("corretto")).unwrap();
        assert!(tools.is_empty());

        let tools = discover_distribution_tools(jdk_path, None).unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_discover_distribution_tools_semeru() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();
        let bin_dir = jdk_path.join("bin");
        fs::create_dir(&bin_dir).unwrap();

        // Create dummy Semeru/OpenJ9 tools
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            for tool in &["jdmpview", "jitserver", "jpackcore", "traceformat"] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();
                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        #[cfg(windows)]
        {
            for tool in &[
                "jdmpview.exe",
                "jitserver.exe",
                "jpackcore.exe",
                "traceformat.exe",
            ] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "test").unwrap();
            }
        }

        let tools = discover_distribution_tools(jdk_path, Some("semeru")).unwrap();

        assert_eq!(tools.len(), 4);
        assert!(tools.contains(&"jdmpview".to_string()));
        assert!(tools.contains(&"jitserver".to_string()));
        assert!(tools.contains(&"jpackcore".to_string()));
        assert!(tools.contains(&"traceformat".to_string()));
    }

    #[test]
    fn test_discover_jdk_tools_sorted_and_unique() {
        let temp_dir = TempDir::new().unwrap();
        let jdk_path = temp_dir.path();
        let bin_dir = jdk_path.join("bin");
        fs::create_dir(&bin_dir).unwrap();

        // Create tools in non-alphabetical order
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            for tool in &["javac", "java", "jar", "javac"] {
                // Duplicate javac
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "#!/bin/sh\necho test").unwrap();
                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        #[cfg(windows)]
        {
            for tool in &["javac.exe", "java.exe", "jar.exe"] {
                let tool_path = bin_dir.join(tool);
                fs::write(&tool_path, "test").unwrap();
            }
        }

        let tools = discover_jdk_tools(jdk_path).unwrap();

        // Should be sorted and deduplicated
        assert_eq!(tools, vec!["jar", "java", "javac"]);
    }
}
