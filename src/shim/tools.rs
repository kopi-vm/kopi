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

use crate::models::distribution::Distribution;
use std::collections::HashMap;

/// Categories of JDK tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Core,       // Essential tools like java, javac
    Debug,      // Debugging tools like jdb, jconsole
    Monitoring, // Performance and monitoring tools
    Security,   // Security-related tools
    Utility,    // Other utilities
}

/// Information about a JDK tool
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub category: ToolCategory,
    pub description: &'static str,
    pub min_version: Option<u32>, // Minimum major version where tool is available
    pub max_version: Option<u32>, // Maximum major version where tool is available
}

/// Type alias for version range (min_version, max_version)
type VersionRange = (Option<u32>, Option<u32>);

/// Type alias for tool exclusions per distribution
type DistributionExclusions = HashMap<&'static str, HashMap<&'static str, VersionRange>>;

/// Registry of all known JDK tools
pub struct ToolRegistry {
    tools: Vec<ToolInfo>,
    distribution_exclusions: DistributionExclusions,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: Self::standard_tools(),
            distribution_exclusions: HashMap::new(),
        };

        // Initialize distribution-specific exclusions
        registry.init_distribution_exclusions();

        registry
    }

    /// Get all available tools
    pub fn all_tools(&self) -> &[ToolInfo] {
        &self.tools
    }

    /// Get tools filtered by category
    pub fn tools_by_category(&self, category: ToolCategory) -> Vec<&ToolInfo> {
        self.tools
            .iter()
            .filter(|tool| tool.category == category)
            .collect()
    }

    /// Get core tools that should be installed by default
    pub fn core_tools(&self) -> Vec<&ToolInfo> {
        self.tools_by_category(ToolCategory::Core)
    }

    /// Check if a tool is available for a specific distribution and version
    pub fn is_tool_available(
        &self,
        tool_name: &str,
        distribution: &Distribution,
        major_version: u32,
    ) -> bool {
        // First check if tool exists in registry
        let tool = match self.tools.iter().find(|t| t.name == tool_name) {
            Some(t) => t,
            None => return false,
        };

        // Check version constraints
        if let Some(min_ver) = tool.min_version {
            if major_version < min_ver {
                return false;
            }
        }

        if let Some(max_ver) = tool.max_version {
            if major_version > max_ver {
                return false;
            }
        }

        // Check distribution-specific exclusions
        if let Some(dist_exclusions) = self.distribution_exclusions.get(distribution.id()) {
            if let Some((min_excl, max_excl)) = dist_exclusions.get(tool_name) {
                // Special handling for "never available" (999, 999)
                if min_excl == &Some(999) && max_excl == &Some(999) {
                    return false;
                }

                // For GraalVM js tool - available before version 23
                if distribution.id() == "graalvm" && tool_name == "js" {
                    if let Some(min) = min_excl {
                        if major_version >= *min {
                            return false;
                        }
                    }
                } else {
                    // Normal exclusion logic
                    if let Some(min) = min_excl {
                        if major_version < *min {
                            return false;
                        }
                    }
                    if let Some(max) = max_excl {
                        if major_version > *max {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Get all tools available for a specific distribution and version
    pub fn available_tools(
        &self,
        distribution: &Distribution,
        major_version: u32,
    ) -> Vec<&ToolInfo> {
        self.tools
            .iter()
            .filter(|tool| self.is_tool_available(tool.name, distribution, major_version))
            .collect()
    }

    /// Get tool info by name
    pub fn get_tool(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    /// Initialize standard JDK tools
    fn standard_tools() -> Vec<ToolInfo> {
        vec![
            // Core tools - available in all JDK versions
            ToolInfo {
                name: "java",
                category: ToolCategory::Core,
                description: "Java application launcher",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "javac",
                category: ToolCategory::Core,
                description: "Java compiler",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "javadoc",
                category: ToolCategory::Core,
                description: "Java documentation generator",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jar",
                category: ToolCategory::Core,
                description: "Java archive tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "javap",
                category: ToolCategory::Core,
                description: "Java class file disassembler",
                min_version: None,
                max_version: None,
            },
            // Debug tools
            ToolInfo {
                name: "jdb",
                category: ToolCategory::Debug,
                description: "Java debugger",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jconsole",
                category: ToolCategory::Debug,
                description: "Java monitoring and management console",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jstack",
                category: ToolCategory::Debug,
                description: "Stack trace tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jmap",
                category: ToolCategory::Debug,
                description: "Memory map tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jhat",
                category: ToolCategory::Debug,
                description: "Heap analysis tool",
                min_version: None,
                max_version: Some(8), // Removed in JDK 9
            },
            ToolInfo {
                name: "jhsdb",
                category: ToolCategory::Debug,
                description: "HotSpot debugger",
                min_version: Some(9),
                max_version: None,
            },
            // Monitoring tools
            ToolInfo {
                name: "jps",
                category: ToolCategory::Monitoring,
                description: "JVM process status tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jstat",
                category: ToolCategory::Monitoring,
                description: "JVM statistics monitoring tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jinfo",
                category: ToolCategory::Monitoring,
                description: "Configuration info tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jcmd",
                category: ToolCategory::Monitoring,
                description: "JVM diagnostic command tool",
                min_version: Some(7),
                max_version: None,
            },
            ToolInfo {
                name: "jfr",
                category: ToolCategory::Monitoring,
                description: "Java Flight Recorder",
                min_version: Some(11),
                max_version: None,
            },
            ToolInfo {
                name: "jstatd",
                category: ToolCategory::Monitoring,
                description: "JVM statistics daemon",
                min_version: None,
                max_version: None,
            },
            // Security tools
            ToolInfo {
                name: "keytool",
                category: ToolCategory::Security,
                description: "Key and certificate management tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jarsigner",
                category: ToolCategory::Security,
                description: "JAR signing and verification tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "policytool",
                category: ToolCategory::Security,
                description: "Policy file creation and management tool",
                min_version: None,
                max_version: Some(10), // Removed in JDK 11
            },
            // Utility tools
            ToolInfo {
                name: "jshell",
                category: ToolCategory::Utility,
                description: "Java shell (REPL)",
                min_version: Some(9),
                max_version: None,
            },
            ToolInfo {
                name: "jlink",
                category: ToolCategory::Utility,
                description: "Java linker",
                min_version: Some(9),
                max_version: None,
            },
            ToolInfo {
                name: "jmod",
                category: ToolCategory::Utility,
                description: "Java module tool",
                min_version: Some(9),
                max_version: None,
            },
            ToolInfo {
                name: "jdeps",
                category: ToolCategory::Utility,
                description: "Java dependency analyzer",
                min_version: Some(8),
                max_version: None,
            },
            ToolInfo {
                name: "jpackage",
                category: ToolCategory::Utility,
                description: "Java packaging tool",
                min_version: Some(14),
                max_version: None,
            },
            ToolInfo {
                name: "serialver",
                category: ToolCategory::Utility,
                description: "Serial version inspector",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "rmiregistry",
                category: ToolCategory::Utility,
                description: "Java RMI registry",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jdeprscan",
                category: ToolCategory::Utility,
                description: "Deprecated API scanner",
                min_version: Some(9),
                max_version: None,
            },
            ToolInfo {
                name: "jimage",
                category: ToolCategory::Utility,
                description: "JDK module image tool",
                min_version: Some(9),
                max_version: None,
            },
            ToolInfo {
                name: "jrunscript",
                category: ToolCategory::Utility,
                description: "Script execution tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "jwebserver",
                category: ToolCategory::Utility,
                description: "Simple web server",
                min_version: Some(18),
                max_version: None,
            },
            ToolInfo {
                name: "native-image",
                category: ToolCategory::Utility,
                description: "GraalVM native image builder",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "native-image-configure",
                category: ToolCategory::Utility,
                description: "GraalVM native image configuration tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "native-image-inspect",
                category: ToolCategory::Utility,
                description: "GraalVM native image inspection tool",
                min_version: None,
                max_version: None,
            },
            ToolInfo {
                name: "js",
                category: ToolCategory::Utility,
                description: "GraalVM JavaScript interpreter",
                min_version: None,
                max_version: Some(22), // Removed in GraalVM 23+
            },
            ToolInfo {
                name: "asprof",
                category: ToolCategory::Monitoring,
                description: "SAP Machine async profiler",
                min_version: None,
                max_version: None,
            },
        ]
    }

    /// Initialize distribution-specific tool exclusions
    fn init_distribution_exclusions(&mut self) {
        // GraalVM-specific tools (only available in GraalVM)
        let graalvm_only_tools = vec![
            "native-image",
            "native-image-configure",
            "native-image-inspect",
            "js",
        ];

        // Add exclusions for non-GraalVM distributions
        for dist in &[
            Distribution::Temurin,
            Distribution::Corretto,
            Distribution::Zulu,
            Distribution::Liberica,
            Distribution::Semeru,
            Distribution::Dragonwell,
            Distribution::SapMachine,
            Distribution::OpenJdk,
            Distribution::Mandrel,
            Distribution::Kona,
            Distribution::Trava,
        ] {
            let mut exclusions = HashMap::new();
            for tool in &graalvm_only_tools {
                // These tools don't exist in non-GraalVM distributions
                exclusions.insert(*tool, (Some(999), Some(999))); // Effectively never available
            }
            self.distribution_exclusions.insert(dist.id(), exclusions);
        }

        // GraalVM removed js in version 23+
        let mut graalvm_exclusions = HashMap::new();
        graalvm_exclusions.insert("js", (Some(23), None));
        self.distribution_exclusions
            .insert(Distribution::GraalVm.id(), graalvm_exclusions);

        // SAP Machine-specific tools (only available in SAP Machine)
        let sapmachine_only_tools = vec!["asprof"];

        // Add exclusions for non-SAP Machine distributions
        for dist in &[
            Distribution::Temurin,
            Distribution::Corretto,
            Distribution::Zulu,
            Distribution::Liberica,
            Distribution::Semeru,
            Distribution::Dragonwell,
            Distribution::GraalVm,
            Distribution::OpenJdk,
            Distribution::Mandrel,
            Distribution::Kona,
            Distribution::Trava,
        ] {
            // Check if the distribution already has exclusions
            if let Some(exclusions) = self.distribution_exclusions.get_mut(dist.id()) {
                // Add SAP Machine-only tools to existing exclusions
                for tool in &sapmachine_only_tools {
                    exclusions.insert(*tool, (Some(999), Some(999)));
                }
            } else {
                // Create new exclusions for this distribution
                let mut exclusions = HashMap::new();
                for tool in &sapmachine_only_tools {
                    exclusions.insert(*tool, (Some(999), Some(999)));
                }
                self.distribution_exclusions.insert(dist.id(), exclusions);
            }
        }
    }
}

/// Get the default set of tools to create shims for
pub fn default_shim_tools() -> Vec<&'static str> {
    vec!["java", "javac", "javadoc", "jar", "jshell"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(!registry.all_tools().is_empty());
    }

    #[test]
    fn test_core_tools() {
        let registry = ToolRegistry::new();
        let core_tools = registry.core_tools();

        assert!(!core_tools.is_empty());
        assert!(core_tools.iter().any(|t| t.name == "java"));
        assert!(core_tools.iter().any(|t| t.name == "javac"));
    }

    #[test]
    fn test_tool_by_name() {
        let registry = ToolRegistry::new();

        let java_tool = registry.get_tool("java").unwrap();
        assert_eq!(java_tool.name, "java");
        assert_eq!(java_tool.category, ToolCategory::Core);

        let nonexistent = registry.get_tool("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_version_constraints() {
        let registry = ToolRegistry::new();

        // jshell is available from JDK 9+
        assert!(!registry.is_tool_available("jshell", &Distribution::Temurin, 8));
        assert!(registry.is_tool_available("jshell", &Distribution::Temurin, 9));
        assert!(registry.is_tool_available("jshell", &Distribution::Temurin, 17));

        // jhat was removed in JDK 9
        assert!(registry.is_tool_available("jhat", &Distribution::Temurin, 8));
        assert!(!registry.is_tool_available("jhat", &Distribution::Temurin, 9));
    }

    #[test]
    fn test_distribution_exclusions() {
        let registry = ToolRegistry::new();

        // native-image is only available in GraalVM
        assert!(registry.is_tool_available("native-image", &Distribution::GraalVm, 21));
        assert!(!registry.is_tool_available("native-image", &Distribution::Temurin, 21));
        assert!(!registry.is_tool_available("native-image", &Distribution::Corretto, 21));

        // js was removed from GraalVM 23+
        assert!(registry.is_tool_available("js", &Distribution::GraalVm, 22));
        assert!(!registry.is_tool_available("js", &Distribution::GraalVm, 23));
        assert!(!registry.is_tool_available("js", &Distribution::Temurin, 21));

        // asprof is only available in SAP Machine
        assert!(registry.is_tool_available("asprof", &Distribution::SapMachine, 21));
        assert!(!registry.is_tool_available("asprof", &Distribution::Temurin, 21));
        assert!(!registry.is_tool_available("asprof", &Distribution::GraalVm, 21));
        assert!(!registry.is_tool_available("asprof", &Distribution::Corretto, 21));
    }

    #[test]
    fn test_available_tools_for_distribution() {
        let registry = ToolRegistry::new();

        // Test Temurin JDK 8
        let temurin8_tools = registry.available_tools(&Distribution::Temurin, 8);
        assert!(temurin8_tools.iter().any(|t| t.name == "java"));
        assert!(temurin8_tools.iter().any(|t| t.name == "jhat")); // Still available in 8
        assert!(!temurin8_tools.iter().any(|t| t.name == "jshell")); // Not yet available
        assert!(!temurin8_tools.iter().any(|t| t.name == "native-image")); // GraalVM only

        // Test GraalVM 21
        let graalvm21_tools = registry.available_tools(&Distribution::GraalVm, 21);
        assert!(graalvm21_tools.iter().any(|t| t.name == "java"));
        assert!(graalvm21_tools.iter().any(|t| t.name == "native-image"));
        assert!(graalvm21_tools.iter().any(|t| t.name == "js"));
        assert!(!graalvm21_tools.iter().any(|t| t.name == "jhat")); // Removed in 9+
    }

    #[test]
    fn test_tools_by_category() {
        let registry = ToolRegistry::new();

        let debug_tools = registry.tools_by_category(ToolCategory::Debug);
        assert!(!debug_tools.is_empty());
        assert!(
            debug_tools
                .iter()
                .all(|t| t.category == ToolCategory::Debug)
        );
        assert!(debug_tools.iter().any(|t| t.name == "jdb"));

        let security_tools = registry.tools_by_category(ToolCategory::Security);
        assert!(!security_tools.is_empty());
        assert!(security_tools.iter().any(|t| t.name == "keytool"));
    }

    #[test]
    fn test_default_shim_tools() {
        let defaults = default_shim_tools();
        assert!(!defaults.is_empty());
        assert!(defaults.contains(&"java"));
        assert!(defaults.contains(&"javac"));
    }
}
