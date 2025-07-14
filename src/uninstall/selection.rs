use crate::storage::InstalledJdk;

pub struct JdkSelector;

impl JdkSelector {
    pub fn filter_by_distribution(
        jdks: Vec<InstalledJdk>,
        distribution: &str,
    ) -> Vec<InstalledJdk> {
        jdks.into_iter()
            .filter(|jdk| jdk.distribution.eq_ignore_ascii_case(distribution))
            .collect()
    }

    pub fn format_selection_summary(jdks: &[InstalledJdk]) -> String {
        if jdks.is_empty() {
            return "No JDKs selected".to_string();
        }

        if jdks.len() == 1 {
            return format!("Selected: {}@{}", jdks[0].distribution, jdks[0].version);
        }

        let distributions: std::collections::HashSet<_> =
            jdks.iter().map(|jdk| jdk.distribution.as_str()).collect();

        if distributions.len() == 1 {
            format!(
                "Selected {} {} versions",
                jdks.len(),
                distributions.iter().next().unwrap()
            )
        } else {
            format!("Selected {} JDKs from multiple distributions", jdks.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::Version;
    use std::path::PathBuf;
    use std::str::FromStr;

    fn create_test_jdk(distribution: &str, version: &str) -> InstalledJdk {
        InstalledJdk {
            distribution: distribution.to_string(),
            version: Version::from_str(version).unwrap(),
            path: PathBuf::from(format!("/test/jdks/{distribution}-{version}")),
        }
    }

    #[test]
    fn test_filter_by_distribution() {
        let jdks = vec![
            create_test_jdk("temurin", "21.0.5+11"),
            create_test_jdk("temurin", "17.0.9+9"),
            create_test_jdk("corretto", "21.0.1"),
            create_test_jdk("corretto", "17.0.5"),
        ];

        let temurin_jdks = JdkSelector::filter_by_distribution(jdks.clone(), "temurin");
        assert_eq!(temurin_jdks.len(), 2);
        assert!(temurin_jdks.iter().all(|jdk| jdk.distribution == "temurin"));

        let corretto_jdks = JdkSelector::filter_by_distribution(jdks.clone(), "corretto");
        assert_eq!(corretto_jdks.len(), 2);
        assert!(
            corretto_jdks
                .iter()
                .all(|jdk| jdk.distribution == "corretto")
        );

        let empty_jdks = JdkSelector::filter_by_distribution(jdks, "zulu");
        assert!(empty_jdks.is_empty());
    }

    #[test]
    fn test_filter_case_insensitive() {
        let jdks = vec![
            create_test_jdk("Temurin", "21.0.5+11"),
            create_test_jdk("TEMURIN", "17.0.9+9"),
            create_test_jdk("temurin", "11.0.21+9"),
        ];

        let filtered = JdkSelector::filter_by_distribution(jdks, "temurin");
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_format_selection_summary() {
        let empty: Vec<InstalledJdk> = vec![];
        assert_eq!(
            JdkSelector::format_selection_summary(&empty),
            "No JDKs selected"
        );

        let single = vec![create_test_jdk("temurin", "21.0.5+11")];
        assert_eq!(
            JdkSelector::format_selection_summary(&single),
            "Selected: temurin@21.0.5+11"
        );

        let multiple_same_dist = vec![
            create_test_jdk("temurin", "21.0.5+11"),
            create_test_jdk("temurin", "17.0.9+9"),
        ];
        assert_eq!(
            JdkSelector::format_selection_summary(&multiple_same_dist),
            "Selected 2 temurin versions"
        );

        let multiple_diff_dist = vec![
            create_test_jdk("temurin", "21.0.5+11"),
            create_test_jdk("corretto", "21.0.1"),
        ];
        assert_eq!(
            JdkSelector::format_selection_summary(&multiple_diff_dist),
            "Selected 2 JDKs from multiple distributions"
        );
    }
}
