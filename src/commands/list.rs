use crate::config::KopiConfig;
use crate::error::Result;
use crate::storage::JdkRepository;
use crate::storage::formatting::format_size;
use log::debug;

pub struct ListCommand<'a> {
    config: &'a KopiConfig,
}

impl<'a> ListCommand<'a> {
    pub fn new(config: &'a KopiConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn execute(&self) -> Result<()> {
        let repository = JdkRepository::new(self.config);

        // List installed JDKs
        let installed_jdks = repository.list_installed_jdks()?;

        if installed_jdks.is_empty() {
            println!("No JDKs installed");
            println!("Use 'kopi install <version>' to install a JDK");
            return Ok(());
        }

        // Calculate disk usage for each JDK and display
        println!("Installed JDKs:");
        let mut total_size = 0u64;

        for jdk in &installed_jdks {
            let size = repository.get_jdk_size(&jdk.path)?;
            total_size += size;

            debug!("JDK {} size: {} bytes", jdk.path.display(), size);

            // Display format: "  temurin@21.0.1 (1.2 GB)"
            println!(
                "  {}@{} ({})",
                jdk.distribution,
                jdk.version,
                format_size(size)
            );
        }

        // Show total disk usage
        println!();
        println!(
            "Total disk usage: {} ({} JDK{})",
            format_size(total_size),
            installed_jdks.len(),
            if installed_jdks.len() == 1 { "" } else { "s" }
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KopiConfig;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_list_no_jdks() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let _repository = JdkRepository::new(&config);

        // Create jdks directory but leave it empty
        fs::create_dir_all(config.jdks_dir().unwrap()).unwrap();

        let command = ListCommand::new(&config).unwrap();

        // This would need proper testing infrastructure to capture stdout
        // For now, we just test that the command can be created and executed
        let result = command.execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_with_jdks() {
        let temp_dir = TempDir::new().unwrap();
        let config = KopiConfig::new(temp_dir.path().to_path_buf()).unwrap();
        let _repository = JdkRepository::new(&config);

        // Create jdks directory
        let jdks_dir = config.jdks_dir().unwrap();
        fs::create_dir_all(&jdks_dir).unwrap();

        // Create mock JDK directories
        let jdk1_path = jdks_dir.join("temurin-21.0.1");
        let jdk2_path = jdks_dir.join("corretto-17.0.9");

        fs::create_dir_all(&jdk1_path).unwrap();
        fs::create_dir_all(&jdk2_path).unwrap();

        // Create some mock files to give the JDKs size
        fs::write(jdk1_path.join("mock_file"), "test content").unwrap();
        fs::write(jdk2_path.join("mock_file"), "test content").unwrap();

        let command = ListCommand::new(&config).unwrap();

        // This would need proper testing infrastructure to capture stdout
        // For now, we just test that the command can be created and executed
        let result = command.execute();
        assert!(result.is_ok());
    }
}
