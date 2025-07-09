#[cfg(test)]
use crate::api::client::ApiClient;
#[cfg(test)]
use crate::api::query::PackageQuery;
#[cfg(test)]
use crate::models::api::*;

#[test]
fn test_api_client_creation() {
    let client = ApiClient::new();
    assert_eq!(client.base_url, "https://api.foojay.io/disco");
}

#[test]
fn test_api_client_with_custom_base_url() {
    let custom_url = "https://test.example.com";
    let client = ApiClient::new().with_base_url(custom_url.to_string());
    assert_eq!(client.base_url, custom_url);
}

#[test]
fn test_package_query_builder() {
    let query = PackageQuery {
        version: Some("21".to_string()),
        distribution: Some("temurin".to_string()),
        architecture: Some("x64".to_string()),
        package_type: Some("jdk".to_string()),
        operating_system: Some("linux".to_string()),
        archive_types: Some(vec!["tar.gz".to_string(), "zip".to_string()]),
        latest: Some("per_version".to_string()),
        directly_downloadable: Some(true),
        lib_c_type: None,
        javafx_bundled: None,
    };

    assert_eq!(query.version, Some("21".to_string()));
    assert_eq!(query.distribution, Some("temurin".to_string()));
    assert_eq!(
        query.archive_types,
        Some(vec!["tar.gz".to_string(), "zip".to_string()])
    );
}

#[test]
fn test_package_query_builder_methods() {
    let query = PackageQuery::new()
        .version("21")
        .distribution("temurin")
        .architecture("x64")
        .package_type("jdk")
        .operating_system("linux")
        .archive_types(vec!["tar.gz".to_string(), "zip".to_string()])
        .latest("per_version")
        .directly_downloadable(true);

    assert_eq!(query.version, Some("21".to_string()));
    assert_eq!(query.distribution, Some("temurin".to_string()));
    assert_eq!(query.architecture, Some("x64".to_string()));
    assert_eq!(query.package_type, Some("jdk".to_string()));
    assert_eq!(query.operating_system, Some("linux".to_string()));
    assert_eq!(
        query.archive_types,
        Some(vec!["tar.gz".to_string(), "zip".to_string()])
    );
    assert_eq!(query.latest, Some("per_version".to_string()));
    assert_eq!(query.directly_downloadable, Some(true));
    assert_eq!(query.lib_c_type, None);
}

#[test]
fn test_package_info_structure() {
    // Test that PackageInfo can be deserialized from expected JSON structure
    let json = r#"{
        "filename": "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
        "direct_download_uri": "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
        "download_site_uri": null,
        "checksum": "5a04c9d9e89e685e56b3780ebec4134c723f6e5e9495513a2f23bf5798a3e70f",
        "checksum_type": "sha256",
        "checksum_uri": "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz.sha256.txt",
        "signature_uri": null
    }"#;

    let package_info: PackageInfo =
        serde_json::from_str(json).expect("Failed to parse PackageInfo");
    assert_eq!(
        package_info.filename,
        "OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz"
    );
    assert_eq!(package_info.checksum_type, "sha256");
    assert_eq!(package_info.checksum.len(), 64); // SHA256 is 64 hex chars
}

#[test]
fn test_parse_distributions_api_response() {
    // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/distributions
    let json_response = r#"{
  "result":
  [{
  "name":"ZuluPrime",
  "api_parameter":"zulu_prime",
  "maintained":true,
  "available":true,
  "build_of_openjdk":true,
  "build_of_graalvm":false,
  "official_uri":"https://www.azul.com/products/prime/stream-download/",
  "synonyms": [
    "zing",
    "ZING",
    "Zing",
    "prime",
    "PRIME",
    "Prime",
    "zuluprime",
    "ZULUPRIME",
    "ZuluPrime",
    "zulu_prime",
    "ZULU_PRIME",
    "Zulu_Prime",
    "zulu prime",
    "ZULU PRIME",
    "Zulu Prime"
  ],
  "versions": [
    "23.0.2+7",
    "23.0.1+11",
    "21.0.7+6",
    "17.0.15+6",
    "11.0.27+6",
    "8.0.452+5"
  ]
},
{
  "name":"Zulu",
  "api_parameter":"zulu",
  "maintained":true,
  "available":true,
  "build_of_openjdk":true,
  "build_of_graalvm":false,
  "official_uri":"https://www.azul.com/downloads/?package=jdk",
  "synonyms": [
    "zulu",
    "ZULU",
    "Zulu",
    "zulucore",
    "ZULUCORE",
    "ZuluCore",
    "zulu_core",
    "ZULU_CORE",
    "Zulu_Core",
    "zulu core",
    "ZULU CORE",
    "Zulu Core"
  ],
  "versions": [
    "25-ea+27",
    "24.0.1+9",
    "23.0.2+7",
    "22.0.2+9",
    "21.0.7+6",
    "17.0.15+21"
  ]
},
{
  "name":"Temurin",
  "api_parameter":"temurin",
  "maintained":true,
  "available":true,
  "build_of_openjdk":true,
  "build_of_graalvm":false,
  "official_uri":"https://adoptium.net/temurin/releases",
  "synonyms": [
    "temurin",
    "Temurin",
    "TEMURIN"
  ],
  "versions": [
    "25-ea+28",
    "24.0.1+9",
    "23.0.2+7",
    "21.0.7+6",
    "17.0.15+21"
  ]
}]
}"#;

    // First test parsing the wrapped response
    let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
    let distributions: Vec<Distribution> =
        serde_json::from_value(wrapped_response["result"].clone()).unwrap();

    assert_eq!(distributions.len(), 3);
    assert_eq!(distributions[0].name, "ZuluPrime");
    assert_eq!(distributions[0].api_parameter, "zulu_prime");
    assert!(distributions[0].maintained);
    assert!(distributions[0].available);
    assert!(distributions[0].build_of_openjdk);
    assert!(!distributions[0].build_of_graalvm);

    assert_eq!(distributions[1].name, "Zulu");
    assert_eq!(distributions[1].api_parameter, "zulu");

    assert_eq!(distributions[2].name, "Temurin");
    assert_eq!(distributions[2].api_parameter, "temurin");
    assert!(distributions[2].versions.contains(&"21.0.7+6".to_string()));
}

#[test]
fn test_parse_packages_api_response() {
    // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/packages?version=21&distribution=temurin&architecture=x64&package_type=jdk&operating_system=linux&latest=available
    let json_response = r#"{
"result":[
{"id":"4c4f879899012ff0a8b2e2117df03b0e","archive_type":"tar.gz","distribution":"temurin","major_version":21,"java_version":"21.0.7+6","distribution_version":"21.0.7","jdk_version":21,"latest_build_available":true,"release_status":"ga","term_of_support":"lts","operating_system":"linux","lib_c_type":"glibc","architecture":"x64","fpu":"unknown","package_type":"jdk","javafx_bundled":false,"directly_downloadable":true,"filename":"OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz","links":{"pkg_info_uri":"https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e","pkg_download_redirect":"https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e/redirect"},"free_use_in_production":true,"tck_tested":"unknown","tck_cert_uri":"","aqavit_certified":"unknown","aqavit_cert_uri":"","size":206919519,"feature":[]},
{"id":"6e5c475d47280d8ce27447611d50c645","archive_type":"tar.gz","distribution":"temurin","major_version":21,"java_version":"21.0.7+6","distribution_version":"21.0.7","jdk_version":21,"latest_build_available":true,"release_status":"ga","term_of_support":"lts","operating_system":"linux","lib_c_type":"musl","architecture":"x64","fpu":"unknown","package_type":"jdk","javafx_bundled":false,"directly_downloadable":true,"filename":"OpenJDK21U-jdk_x64_alpine-linux_hotspot_21.0.7_6.tar.gz","links":{"pkg_info_uri":"https://api.foojay.io/disco/v3.0/ids/6e5c475d47280d8ce27447611d50c645","pkg_download_redirect":"https://api.foojay.io/disco/v3.0/ids/6e5c475d47280d8ce27447611d50c645/redirect"},"free_use_in_production":true,"tck_tested":"unknown","tck_cert_uri":"","aqavit_certified":"unknown","aqavit_cert_uri":"","size":207113831,"feature":[]}
],
"message":"2 package(s) found"}"#;

    // Test parsing the wrapped response
    let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
    let packages: Vec<Package> =
        serde_json::from_value(wrapped_response["result"].clone()).unwrap();

    assert_eq!(packages.len(), 2);

    // Test first package (glibc)
    assert_eq!(packages[0].id, "4c4f879899012ff0a8b2e2117df03b0e");
    assert_eq!(packages[0].archive_type, "tar.gz");
    assert_eq!(packages[0].distribution, "temurin");
    assert_eq!(packages[0].major_version, 21);
    assert_eq!(packages[0].java_version, "21.0.7+6");
    assert_eq!(packages[0].distribution_version, "21.0.7");
    assert_eq!(packages[0].jdk_version, 21);
    assert!(packages[0].directly_downloadable);
    assert_eq!(
        packages[0].filename,
        "OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"
    );
    assert_eq!(packages[0].operating_system, "linux");
    assert_eq!(packages[0].lib_c_type, Some("glibc".to_string()));
    assert_eq!(packages[0].size, 206919519);
    assert!(packages[0].free_use_in_production);
    assert_eq!(
        packages[0].links.pkg_download_redirect,
        "https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e/redirect"
    );

    // Test second package (musl)
    assert_eq!(packages[1].id, "6e5c475d47280d8ce27447611d50c645");
    assert_eq!(packages[1].lib_c_type, Some("musl".to_string()));
    assert_eq!(
        packages[1].filename,
        "OpenJDK21U-jdk_x64_alpine-linux_hotspot_21.0.7_6.tar.gz"
    );
}

#[test]
fn test_parse_package_info_by_id_response() {
    // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/ids/4c4f879899012ff0a8b2e2117df03b0e
    let json_response = r#"{
  "result":[
    {
    "filename":"OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz",
    "direct_download_uri":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz",
    "download_site_uri":"",
    "signature_uri":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sig",
    "checksum_uri":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sha256.txt",
    "checksum":"974d3acef0b7193f541acb61b76e81670890551366625d4f6ca01b91ac152ce0",
    "checksum_type":"sha256"
  }
    ],
  "message":""
}"#;

    // Test parsing the wrapped response
    let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
    let package_infos: Vec<PackageInfo> =
        serde_json::from_value(wrapped_response["result"].clone()).unwrap();

    assert_eq!(package_infos.len(), 1);
    let package_info = &package_infos[0];

    assert_eq!(
        package_info.filename,
        "OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"
    );
    assert_eq!(
        package_info.direct_download_uri,
        "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz"
    );
    assert_eq!(package_info.download_site_uri, Some("".to_string()));
    assert_eq!(package_info.signature_uri, Some("https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sig".to_string()));
    assert_eq!(
        package_info.checksum_uri,
        "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz.sha256.txt"
    );
    assert_eq!(
        package_info.checksum,
        "974d3acef0b7193f541acb61b76e81670890551366625d4f6ca01b91ac152ce0"
    );
    assert_eq!(package_info.checksum_type, "sha256");
    assert_eq!(package_info.checksum.len(), 64); // SHA256 is 64 hex chars
}

#[test]
fn test_parse_major_versions_api_response() {
    // JSON response obtained from: curl https://api.foojay.io/disco/v3.0/major_versions
    let json_response = r#"{
  "result":[
    {
      "major_version":24,
      "term_of_support":"STS",
      "maintained":true,
      "early_access_only":false,
      "release_status":"ga",
      "versions": [
        "24.0.1+11",
        "24.0.1+9",
        "24.0.1",
        "24+37",
        "24+36"
      ]
    },
    {
      "major_version":21,
      "term_of_support":"LTS",
      "maintained":true,
      "early_access_only":false,
      "release_status":"ga",
      "versions": [
        "21.0.7+6",
        "21.0.6+7",
        "21.0.5+11",
        "21.0.4+7",
        "21.0.3+9"
      ]
    },
    {
      "major_version":17,
      "term_of_support":"LTS",
      "maintained":true,
      "early_access_only":false,
      "release_status":"ga",
      "versions": [
        "17.0.15+21",
        "17.0.14+7",
        "17.0.13+11",
        "17.0.12+7",
        "17.0.11+9"
      ]
    }
  ]
}"#;

    // Test parsing the wrapped response
    let wrapped_response: serde_json::Value = serde_json::from_str(json_response).unwrap();
    let major_versions: Vec<MajorVersion> =
        serde_json::from_value(wrapped_response["result"].clone()).unwrap();

    assert_eq!(major_versions.len(), 3);

    // Test first version (24 - STS)
    assert_eq!(major_versions[0].major_version, 24);
    assert_eq!(major_versions[0].term_of_support, "STS");
    assert!(
        major_versions[0]
            .versions
            .contains(&"24.0.1+11".to_string())
    );

    // Test second version (21 - LTS)
    assert_eq!(major_versions[1].major_version, 21);
    assert_eq!(major_versions[1].term_of_support, "LTS");
    assert!(major_versions[1].versions.contains(&"21.0.7+6".to_string()));

    // Test third version (17 - LTS)
    assert_eq!(major_versions[2].major_version, 17);
    assert_eq!(major_versions[2].term_of_support, "LTS");
    assert!(
        major_versions[2]
            .versions
            .contains(&"17.0.15+21".to_string())
    );
}

#[test]
fn test_package_model_with_new_fields() {
    // Test that Package model correctly serializes/deserializes with new fields
    let package = Package {
        id: "test-id".to_string(),
        archive_type: "tar.gz".to_string(),
        distribution: "temurin".to_string(),
        major_version: 21,
        java_version: "21.0.1".to_string(),
        distribution_version: "21.0.1+12".to_string(),
        jdk_version: 21,
        directly_downloadable: true,
        filename: "test.tar.gz".to_string(),
        links: Links {
            pkg_download_redirect: "https://example.com/download".to_string(),
            pkg_info_uri: Some("https://example.com/info".to_string()),
        },
        free_use_in_production: true,
        tck_tested: "yes".to_string(),
        size: 100000000,
        operating_system: "linux".to_string(),
        lib_c_type: Some("glibc".to_string()),
        package_type: "jdk".to_string(),
        javafx_bundled: false,
        term_of_support: Some("lts".to_string()),
        release_status: Some("ga".to_string()),
        latest_build_available: Some(true),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&package).unwrap();

    // Deserialize back
    let deserialized: Package = serde_json::from_str(&json).unwrap();

    // Verify new fields
    assert_eq!(deserialized.term_of_support, Some("lts".to_string()));
    assert_eq!(deserialized.release_status, Some("ga".to_string()));
    assert_eq!(deserialized.latest_build_available, Some(true));
}

#[test]
fn test_package_model_optional_fields() {
    // Test that Package model works when new fields are None
    let json = r#"{
        "id": "test-id",
        "archive_type": "tar.gz",
        "distribution": "temurin",
        "major_version": 21,
        "java_version": "21.0.1",
        "distribution_version": "21.0.1+12",
        "jdk_version": 21,
        "directly_downloadable": true,
        "filename": "test.tar.gz",
        "links": {
            "pkg_download_redirect": "https://example.com/download"
        },
        "free_use_in_production": true,
        "tck_tested": "yes",
        "size": 100000000,
        "operating_system": "linux",
        "package_type": "jdk",
        "javafx_bundled": false
    }"#;

    let package: Package = serde_json::from_str(json).unwrap();

    // Verify optional fields are None when not present
    assert_eq!(package.term_of_support, None);
    assert_eq!(package.release_status, None);
    assert_eq!(package.latest_build_available, None);
    assert_eq!(package.lib_c_type, None);
}
