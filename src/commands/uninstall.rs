use crate::config::KopiConfig;
use crate::error::{KopiError, Result};
use crate::storage::JdkRepository;
use crate::uninstall::UninstallHandler;
use crate::uninstall::batch::BatchUninstaller;
use crate::uninstall::cleanup::UninstallCleanup;
use crate::uninstall::feedback::{display_uninstall_confirmation, display_uninstall_summary};
use crate::version::VersionRequest;
use log::{debug, info};
use std::str::FromStr;

pub struct UninstallCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> UninstallCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(
        &self,
        version_spec: Option<&str>,
        force: bool,
        dry_run: bool,
        all: bool,
        cleanup: bool,
    ) -> Result<()> {
        debug!("Uninstall options: force={force}, dry_run={dry_run}, all={all}, cleanup={cleanup}");

        let repository = JdkRepository::new(self.config);
        let handler = UninstallHandler::new(&repository);

        // Execute normal uninstall if version is specified
        if let Some(version) = version_spec {
            info!("Uninstall command: {version}");
            if all {
                // Batch uninstall all versions of a distribution
                self.execute_batch_uninstall(version, force, dry_run, self.config, &repository)?;
            } else {
                // Single JDK uninstall
                self.execute_single_uninstall(version, force, dry_run, &handler, &repository)?;
            }
        } else if !cleanup {
            // If no version specified and no cleanup flag, it's an error
            return Err(KopiError::InvalidVersionFormat(
                "Either specify a version to uninstall or use --cleanup flag".to_string(),
            ));
        }

        // Execute cleanup if flag is set
        if cleanup {
            info!("Performing cleanup of failed uninstall operations");
            self.execute_cleanup(force, dry_run, &handler)?;
        }

        Ok(())
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

    fn execute_cleanup(
        &self,
        force: bool,
        dry_run: bool,
        handler: &UninstallHandler,
    ) -> Result<()> {
        info!("Executing cleanup of failed uninstall operations");

        if dry_run {
            // For dry-run, we need to create the cleanup handler ourselves
            let repository = JdkRepository::new(self.config);
            let cleanup = UninstallCleanup::new(&repository);
            let actions = cleanup.detect_and_cleanup_partial_removals()?;

            if actions.is_empty() {
                println!("No cleanup actions needed.");
                return Ok(());
            }

            println!("Would perform the following cleanup actions:");
            for action in &actions {
                println!("  - {action:?}");
            }

            return Ok(());
        }

        // Perform the actual cleanup
        handler.recover_from_failures(force)
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
