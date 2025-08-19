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

//! Integration tests for flexible version matching with build numbers
//! Tests that patterns like "corretto@24.0.2+12" match installed "corretto-24.0.2.12.1"

#[cfg(test)]
mod flexible_version_matching_tests {
    use kopi::storage::JdkRepository;
    use kopi::version::VersionRequest;
    use std::fs;
    use std::str::FromStr;
    use tempfile::TempDir;

    #[test]
    fn test_corretto_flexible_build_matching() {
        // Create test environment
        let temp_dir = TempDir::new().unwrap();
        let config = kopi::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let jdks_dir = config.jdks_dir().unwrap();

        // Create JDK directories that simulate Corretto's version format
        fs::create_dir_all(jdks_dir.join("corretto-24.0.2.12.1")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-21.0.5.11.1")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-17.0.13.11.1")).unwrap();

        let repository = JdkRepository::new(&config);

        // Test 1: corretto@24.0.2+12 should match corretto-24.0.2.12.1
        let request = VersionRequest::from_str("corretto@24.0.2+12").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "corretto");
        assert_eq!(matches[0].version.to_string(), "24.0.2.12.1");

        // Test 2: corretto@21.0.5+11 should match corretto-21.0.5.11.1
        let request = VersionRequest::from_str("corretto@21.0.5+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "corretto");
        assert_eq!(matches[0].version.to_string(), "21.0.5.11.1");

        // Test 3: corretto@17.0.13+11 should match corretto-17.0.13.11.1
        let request = VersionRequest::from_str("corretto@17.0.13+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "corretto");
        assert_eq!(matches[0].version.to_string(), "17.0.13.11.1");

        // Test 4: corretto@24.0.2+13 should NOT match corretto-24.0.2.12.1
        let request = VersionRequest::from_str("corretto@24.0.2+13").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_zulu_flexible_build_matching() {
        // Create test environment
        let temp_dir = TempDir::new().unwrap();
        let config = kopi::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let jdks_dir = config.jdks_dir().unwrap();

        // Create JDK directories that simulate Zulu's version format
        fs::create_dir_all(jdks_dir.join("zulu-21.0.5.11")).unwrap();
        fs::create_dir_all(jdks_dir.join("zulu-21.0.5.11.0.25")).unwrap();
        fs::create_dir_all(jdks_dir.join("zulu-17.0.13.11")).unwrap();

        let repository = JdkRepository::new(&config);

        // Test 1: zulu@21.0.5+11 should match both zulu-21.0.5.11 and zulu-21.0.5.11.0.25
        let request = VersionRequest::from_str("zulu@21.0.5+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].distribution, "zulu");
        assert_eq!(matches[0].version.to_string(), "21.0.5.11");
        assert_eq!(matches[1].distribution, "zulu");
        assert_eq!(matches[1].version.to_string(), "21.0.5.11.0.25");

        // Test 2: zulu@17.0.13+11 should match zulu-17.0.13.11
        let request = VersionRequest::from_str("zulu@17.0.13+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "zulu");
        assert_eq!(matches[0].version.to_string(), "17.0.13.11");
    }

    #[test]
    fn test_mixed_distributions_flexible_matching() {
        // Create test environment
        let temp_dir = TempDir::new().unwrap();
        let config = kopi::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let jdks_dir = config.jdks_dir().unwrap();

        // Create JDK directories for different distributions
        fs::create_dir_all(jdks_dir.join("corretto-21.0.5.11.1")).unwrap();
        fs::create_dir_all(jdks_dir.join("zulu-21.0.5.11")).unwrap();
        fs::create_dir_all(jdks_dir.join("temurin-21.0.5")).unwrap(); // Standard format with build in directory name
        fs::create_dir_all(jdks_dir.join("liberica-21.0.5.11")).unwrap();

        let repository = JdkRepository::new(&config);

        // Test: Each distribution with 21.0.5+11 should match their respective installations

        // Corretto
        let request = VersionRequest::from_str("corretto@21.0.5+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "corretto");

        // Zulu
        let request = VersionRequest::from_str("zulu@21.0.5+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "zulu");

        // Liberica
        let request = VersionRequest::from_str("liberica@21.0.5+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "liberica");

        // Temurin should not match since it doesn't have build 11
        let request = VersionRequest::from_str("temurin@21.0.5+11").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 0);

        // But temurin@21.0.5 should match
        let request = VersionRequest::from_str("temurin@21.0.5").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].distribution, "temurin");
    }

    #[test]
    fn test_version_file_format_preservation() {
        // Test that version files can be read and matched correctly
        let temp_dir = TempDir::new().unwrap();
        let config = kopi::config::KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let jdks_dir = config.jdks_dir().unwrap();

        // Create JDK directories
        fs::create_dir_all(jdks_dir.join("corretto-24.0.2.12")).unwrap();
        fs::create_dir_all(jdks_dir.join("corretto-24.0.2.12.1")).unwrap();

        let repository = JdkRepository::new(&config);

        // Test that corretto@24.0.2+12 matches both 4 and 5 component versions
        let request = VersionRequest::from_str("corretto@24.0.2+12").unwrap();
        let matches = repository.find_matching_jdks(&request).unwrap();
        assert_eq!(matches.len(), 2);

        // The 4-component version should match
        assert!(
            matches
                .iter()
                .any(|jdk| jdk.version.to_string() == "24.0.2.12")
        );

        // The 5-component version should also match
        assert!(
            matches
                .iter()
                .any(|jdk| jdk.version.to_string() == "24.0.2.12.1")
        );
    }
}
