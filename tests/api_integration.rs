use kopi::api::{ApiClient, PackageQuery};
use std::time::Duration;

#[test]
#[ignore]
fn test_get_distributions_real_api() {
    let client = ApiClient::new();
    let result = client.get_distributions();

    assert!(
        result.is_ok(),
        "Failed to fetch distributions: {:?}",
        result
    );
    let distributions = result.unwrap();
    assert!(!distributions.is_empty(), "No distributions returned");

    let has_temurin = distributions.iter().any(|d| d.api_parameter == "temurin");
    assert!(has_temurin, "Expected to find Temurin distribution");
}

#[test]
#[ignore]
fn test_get_major_versions_real_api() {
    let client = ApiClient::new();
    let result = client.get_major_versions();

    assert!(
        result.is_ok(),
        "Failed to fetch major versions: {:?}",
        result
    );
    let versions = result.unwrap();
    assert!(!versions.is_empty(), "No major versions returned");

    let has_v21 = versions.iter().any(|v| v.major_version == 21);
    assert!(has_v21, "Expected to find Java 21");
}

#[test]
#[ignore]
fn test_get_packages_with_query_real_api() {
    let client = ApiClient::new();
    let query = PackageQuery {
        version: Some("21".to_string()),
        distribution: Some("temurin".to_string()),
        architecture: Some("x64".to_string()),
        package_type: Some("jdk".to_string()),
        operating_system: Some("linux".to_string()),
        archive_type: Some("tar.gz".to_string()),
        latest: Some(true),
        directly_downloadable: Some(true),
        lib_c_type: None,
    };

    let result = client.get_packages(Some(query));

    assert!(result.is_ok(), "Failed to fetch packages: {:?}", result);
    let packages = result.unwrap();
    assert!(!packages.is_empty(), "No packages returned for query");

    let package = &packages[0];
    assert_eq!(package.distribution, "temurin");
    assert_eq!(package.major_version, 21);
    assert!(package.directly_downloadable);
}

#[test]
#[ignore]
fn test_timeout_handling() {
    let client = ApiClient::new().with_timeout(Duration::from_millis(1));

    let result = client.get_distributions();
    assert!(result.is_err(), "Expected timeout error");
}

#[cfg(test)]
mod mock_tests {

    #[test]
    fn test_distributions_parsing() {
        let mock_data = r#"[
            {
                "id": "temurin",
                "name": "Eclipse Temurin",
                "api_parameter": "temurin",
                "free_use_in_production": true,
                "synonyms": ["adoptopenjdk", "adopt"],
                "versions": ["8", "11", "17", "21"]
            },
            {
                "id": "corretto",
                "name": "Amazon Corretto",
                "api_parameter": "corretto",
                "free_use_in_production": true,
                "synonyms": [],
                "versions": ["8", "11", "17", "21"]
            }
        ]"#;

        let distributions: Vec<kopi::api::Distribution> = serde_json::from_str(mock_data).unwrap();
        assert_eq!(distributions.len(), 2);
        assert_eq!(distributions[0].api_parameter, "temurin");
        assert_eq!(distributions[1].api_parameter, "corretto");
    }

    #[test]
    fn test_major_versions_parsing() {
        let mock_data = r#"[
            {
                "major_version": 21,
                "term_of_support": "lts",
                "versions": ["21", "21.0.1", "21.0.2"]
            },
            {
                "major_version": 22,
                "term_of_support": "sts",
                "versions": ["22", "22.0.1"]
            }
        ]"#;

        let versions: Vec<kopi::api::MajorVersion> = serde_json::from_str(mock_data).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].major_version, 21);
        assert_eq!(versions[0].term_of_support, "lts");
    }

    #[test]
    fn test_package_parsing() {
        let mock_data = r#"{
            "id": "abc123",
            "archive_type": "tar.gz",
            "distribution": "temurin",
            "major_version": 21,
            "java_version": "21.0.1+12",
            "distribution_version": "21.0.1+12",
            "jdk_version": 21,
            "directly_downloadable": true,
            "filename": "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
            "links": {
                "pkg_download_redirect": "https://api.foojay.io/download/abc123",
                "pkg_info_uri": "https://adoptium.net"
            },
            "free_use_in_production": true,
            "tck_tested": "yes",
            "size": 195000000,
            "operating_system": "linux"
        }"#;

        let package: kopi::api::Package = serde_json::from_str(mock_data).unwrap();
        assert_eq!(package.distribution, "temurin");
        assert_eq!(package.major_version, 21);
        assert_eq!(package.java_version, "21.0.1+12");
        assert!(package.directly_downloadable);
        assert_eq!(package.size, 195000000);
    }

    #[test]
    fn test_empty_response_handling() {
        let empty_array = "[]";
        let distributions: Vec<kopi::api::Distribution> =
            serde_json::from_str(empty_array).unwrap();
        assert!(distributions.is_empty());
    }

    #[test]
    fn test_invalid_json_handling() {
        let invalid_json = "{ invalid json }";
        let result = serde_json::from_str::<Vec<kopi::api::Distribution>>(invalid_json);
        assert!(result.is_err());
    }
}
