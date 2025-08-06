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

//! User-Agent string constants and utilities for consistent HTTP client identification.
//!
//! All HTTP clients in the Kopi codebase should use these constants to ensure
//! consistent User-Agent headers across different features.

/// The Kopi package version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// User-Agent for the API client (foojay.io API calls)
pub fn api_client() -> String {
    format!("kopi/api/{VERSION}")
}

/// User-Agent for metadata HTTP client
pub fn metadata_client() -> String {
    format!("kopi/metadata/{VERSION}")
}

/// User-Agent for download client
pub fn download_client() -> String {
    format!("kopi/download/{VERSION}")
}

/// User-Agent for doctor diagnostic checks
pub fn doctor_client() -> String {
    format!("kopi/doctor/{VERSION}")
}

/// Get a User-Agent string for a specific feature
pub fn for_feature(feature: &str) -> String {
    format!("kopi/{feature}/{VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agents() {
        // Check format of each user agent
        assert_eq!(api_client(), format!("kopi/api/{VERSION}"));
        assert_eq!(metadata_client(), format!("kopi/metadata/{VERSION}"));
        assert_eq!(download_client(), format!("kopi/download/{VERSION}"));
        assert_eq!(doctor_client(), format!("kopi/doctor/{VERSION}"));

        // Test custom feature
        assert_eq!(for_feature("custom"), format!("kopi/custom/{VERSION}"));
    }

    #[test]
    fn test_version_format() {
        // Version should follow semver format (e.g., "0.1.0")
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "Version should have 3 parts (major.minor.patch)"
        );

        // Each part should be a valid number
        for part in parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "Version part should be a number"
            );
        }
    }
}
