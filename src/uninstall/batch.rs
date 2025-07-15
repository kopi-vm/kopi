use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::storage::{InstalledJdk, JdkRepository};
use crate::uninstall::feedback::{
    display_batch_uninstall_confirmation, display_batch_uninstall_summary,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, warn};
use std::time::Duration;

pub struct BatchUninstaller<'a> {
    repository: &'a JdkRepository<'a>,
}

impl<'a> BatchUninstaller<'a> {
    pub fn new(_config: &KopiConfig, repository: &'a JdkRepository<'a>) -> Self {
        Self { repository }
    }

    pub fn uninstall_all(&self, spec: Option<&str>, force: bool, dry_run: bool) -> Result<()> {
        // List all installed JDKs
        let mut jdks = self.repository.list_installed_jdks()?;

        // Filter by distribution or version if specified
        if let Some(spec_str) = spec {
            // Check if spec looks like a version (starts with a digit)
            if spec_str
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                // Filter by version prefix
                jdks.retain(|jdk| jdk.version.to_string().starts_with(spec_str));
            } else {
                // Filter by distribution
                jdks.retain(|jdk| jdk.distribution.eq_ignore_ascii_case(spec_str));
            }
        }

        if jdks.is_empty() {
            return Err(KopiError::JdkNotInstalled {
                jdk_spec: spec.unwrap_or("all").to_string(),
                version: None,
                distribution: None,
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            });
        }

        self.uninstall_batch(jdks, force, dry_run)
    }

    pub fn uninstall_batch(
        &self,
        jdks: Vec<InstalledJdk>,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        if jdks.is_empty() {
            return Ok(());
        }

        // Calculate total size
        let total_size = self.calculate_total_size(&jdks)?;

        // Show summary
        self.display_batch_summary(&jdks, total_size)?;

        if dry_run {
            return Ok(());
        }

        // Confirm unless forced
        if !force && !self.confirm_batch_removal(&jdks)? {
            return Err(KopiError::SystemError(
                "User cancelled operation".to_string(),
            ));
        }

        // Perform batch removal with transaction-like behavior
        self.execute_batch_removal(jdks, total_size)
    }

    fn calculate_total_size(&self, jdks: &[InstalledJdk]) -> Result<u64> {
        let mut total = 0u64;
        for jdk in jdks {
            total += self.repository.get_jdk_size(&jdk.path)?;
        }
        Ok(total)
    }

    fn display_batch_summary(&self, jdks: &[InstalledJdk], total_size: u64) -> Result<()> {
        println!("JDKs to be removed:");
        println!();

        for jdk in jdks {
            let size = self.repository.get_jdk_size(&jdk.path)?;
            println!(
                "  - {}@{} ({})",
                jdk.distribution,
                jdk.version,
                format_size(size)
            );
        }

        println!();
        println!("Total: {} JDKs, {}", jdks.len(), format_size(total_size));

        Ok(())
    }

    fn confirm_batch_removal(&self, jdks: &[InstalledJdk]) -> Result<bool> {
        let total_size = self.calculate_total_size(jdks)?;
        display_batch_uninstall_confirmation(jdks, total_size)
    }

    fn execute_batch_removal(&self, jdks: Vec<InstalledJdk>, total_size: u64) -> Result<()> {
        let multi_progress = MultiProgress::new();
        let overall_pb = multi_progress.add(ProgressBar::new(jdks.len() as u64));
        overall_pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} JDKs removed")
                .unwrap()
                .progress_chars("#>-"),
        );
        overall_pb.set_message("Removing JDKs...");

        let mut removed_count = 0;
        let mut failed_jdks = Vec::new();
        let mut removed_jdks = Vec::new();

        for jdk in &jdks {
            info!("Removing {}@{}", jdk.distribution, jdk.version);

            // Create spinner for current JDK
            let spinner = multi_progress.add(ProgressBar::new_spinner());
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
            spinner.set_message(format!("Removing {}@{}...", jdk.distribution, jdk.version));
            spinner.enable_steady_tick(Duration::from_millis(100));

            // Perform safety checks
            match crate::uninstall::safety::perform_safety_checks(
                &jdk.distribution.to_string(),
                &jdk.version.to_string(),
            ) {
                Ok(()) => {}
                Err(e) => {
                    warn!(
                        "Safety check failed for {}@{}: {}",
                        jdk.distribution, jdk.version, e
                    );
                    spinner.finish_and_clear();
                    failed_jdks.push((jdk.clone(), e));
                    continue;
                }
            }

            // Attempt removal
            match self.repository.remove_jdk(&jdk.path) {
                Ok(()) => {
                    removed_count += 1;
                    removed_jdks.push(jdk.clone());
                    spinner.finish_with_message(format!(
                        "✓ Removed {}@{}",
                        jdk.distribution, jdk.version
                    ));
                    overall_pb.inc(1);
                }
                Err(e) => {
                    warn!(
                        "Failed to remove {}@{}: {}",
                        jdk.distribution, jdk.version, e
                    );
                    spinner.finish_with_message(format!(
                        "✗ Failed to remove {}@{}",
                        jdk.distribution, jdk.version
                    ));
                    failed_jdks.push((jdk.clone(), e));
                }
            }
        }

        overall_pb.finish_and_clear();

        // Report results
        let failed_with_messages: Vec<(InstalledJdk, String)> = failed_jdks
            .into_iter()
            .map(|(jdk, err)| (jdk, err.to_string()))
            .collect();

        display_batch_uninstall_summary(&removed_jdks, &failed_with_messages, total_size);

        // Return error if all removals failed
        if removed_count == 0 {
            return Err(KopiError::SystemError(
                "All JDK removals failed".to_string(),
            ));
        }

        Ok(())
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use crate::version::Version;
    use mockall::mock;
    use std::path::PathBuf;
    use std::str::FromStr;
    use tempfile::TempDir;

    // Mock JdkRepository trait
    mock! {
        JdkRepo {
            fn list_installed_jdks(&self) -> Result<Vec<InstalledJdk>>;
            fn get_jdk_size(&self, path: &std::path::Path) -> Result<u64>;
            fn remove_jdk(&self, path: &std::path::Path) -> Result<()>;
        }
    }

    fn create_test_jdk(distribution: &str, version: &str, path: &str) -> InstalledJdk {
        InstalledJdk {
            distribution: distribution.to_string(),
            version: Version::from_str(version).unwrap(),
            path: PathBuf::from(path),
        }
    }

    #[test]
    fn test_calculate_total_size() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let repository = JdkRepository::new(&config);
        let batch_uninstaller = BatchUninstaller::new(&config, &repository);

        // Create some test directories
        let jdk1_path = temp_dir.path().join("jdks").join("temurin-21.0.5+11");
        let jdk2_path = temp_dir.path().join("jdks").join("corretto-17.0.9");
        std::fs::create_dir_all(&jdk1_path).unwrap();
        std::fs::create_dir_all(&jdk2_path).unwrap();

        // Write some test files
        std::fs::write(jdk1_path.join("test1.txt"), vec![0u8; 1024]).unwrap();
        std::fs::write(jdk2_path.join("test2.txt"), vec![0u8; 2048]).unwrap();

        let jdks = vec![
            create_test_jdk("temurin", "21.0.5+11", jdk1_path.to_str().unwrap()),
            create_test_jdk("corretto", "17.0.9", jdk2_path.to_str().unwrap()),
        ];

        let total_size = batch_uninstaller.calculate_total_size(&jdks).unwrap();
        assert_eq!(total_size, 3072);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }
}
