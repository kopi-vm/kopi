use crate::config::new_kopi_config;
use crate::error::{KopiError, Result};
use crate::storage::JdkRepository;
use crate::uninstall::UninstallHandler;
use crate::uninstall::batch::BatchUninstaller;
use crate::uninstall::feedback::{display_uninstall_confirmation, display_uninstall_summary};
use crate::version::VersionRequest;
use log::{debug, info};
use std::str::FromStr;

pub struct UninstallCommand;

impl UninstallCommand {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn execute(&self, version_spec: &str, force: bool, dry_run: bool, all: bool) -> Result<()> {
        info!("Uninstall command: {version_spec}");
        debug!("Uninstall options: force={force}, dry_run={dry_run}, all={all}");

        let config = new_kopi_config()?;
        let repository = JdkRepository::new(&config);
        let handler = UninstallHandler::new(&repository);

        if all {
            // Batch uninstall all versions of a distribution
            self.execute_batch_uninstall(version_spec, force, dry_run, &config, &repository)
        } else {
            // Single JDK uninstall
            self.execute_single_uninstall(version_spec, force, dry_run, &handler, &repository)
        }
    }

    fn execute_single_uninstall(
        &self,
        version_spec: &str,
        force: bool,
        dry_run: bool,
        handler: &UninstallHandler,
        repository: &JdkRepository,
    ) -> Result<()> {
        // Parse version specification using lenient parsing
        let version_request = VersionRequest::from_str(version_spec)?;
        debug!("Parsed version request: {version_request:?}");

        // Get JDKs to uninstall using find_matching_jdks
        let jdks_to_remove = repository.find_matching_jdks(&version_request)?;

        if jdks_to_remove.is_empty() {
            return Err(crate::error::KopiError::JdkNotInstalled {
                jdk_spec: version_spec.to_string(),
                version: None,
                distribution: None,
                auto_install_enabled: false,
                auto_install_failed: None,
                user_declined: false,
                install_in_progress: false,
            });
        }

        // Handle multiple matches
        if jdks_to_remove.len() > 1 {
            // Return error when multiple JDKs match to avoid ambiguity
            let jdk_list: Vec<String> = jdks_to_remove
                .iter()
                .map(|j| format!("  - {}@{}", j.distribution, j.version))
                .collect();

            eprintln!("Error: Multiple JDKs match the pattern '{version_spec}'");
            eprintln!("\nFound the following JDKs:");
            for jdk in &jdk_list {
                eprintln!("{jdk}");
            }
            eprintln!("\nPlease specify which JDK to uninstall:");
            eprintln!("  kopi uninstall <distribution>@<version>");
            eprintln!("\nExamples:");
            eprintln!("  kopi uninstall temurin@21.0.5+11");
            eprintln!("  kopi uninstall corretto@17.0.13.11.1");

            return Err(KopiError::SystemError(
                "Multiple JDKs match the specified pattern".to_string(),
            ));
        }

        let jdk = &jdks_to_remove[0];

        // Calculate disk space to be freed
        let disk_space = repository.get_jdk_size(&jdk.path)?;

        // Display confirmation prompt (unless --force)
        if !force && !dry_run && !display_uninstall_confirmation(jdk, disk_space)? {
            println!("Uninstall cancelled.");
            return Ok(());
        }

        if dry_run {
            println!("Would uninstall: {}@{}", jdk.distribution, jdk.version);
            println!("Would free: {:.2} MB", disk_space as f64 / 1_048_576.0);
            return Ok(());
        }

        // Perform the uninstall
        handler.uninstall_jdk(version_spec, false)?;

        // Display success summary
        display_uninstall_summary(&[jdk.clone()], disk_space);

        Ok(())
    }

    fn execute_batch_uninstall(
        &self,
        distribution_spec: &str,
        force: bool,
        dry_run: bool,
        config: &crate::config::KopiConfig,
        repository: &JdkRepository,
    ) -> Result<()> {
        let batch_uninstaller = BatchUninstaller::new(config, repository);
        batch_uninstaller.uninstall_all(Some(distribution_spec), force, dry_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uninstall_command_creation() {
        let command = UninstallCommand::new();
        assert!(command.is_ok());
    }
}
