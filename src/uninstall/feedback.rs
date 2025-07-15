use crate::error::Result;
use crate::storage::InstalledJdk;
use std::io::{self, Write};

/// Display confirmation prompt for uninstalling a JDK
pub fn display_uninstall_confirmation(jdk: &InstalledJdk, disk_space: u64) -> Result<bool> {
    println!("The following JDK will be uninstalled:");
    println!("  Distribution: {}", jdk.distribution);
    println!("  Version: {}", jdk.version);
    println!("  Path: {}", jdk.path.display());
    println!(
        "  Disk space to be freed: {:.2} MB",
        disk_space as f64 / 1_048_576.0
    );
    println!();

    print!("Do you want to continue? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Display confirmation prompt for batch uninstall
pub fn display_batch_uninstall_confirmation(
    jdks: &[InstalledJdk],
    total_disk_space: u64,
) -> Result<bool> {
    println!("The following {} JDK(s) will be uninstalled:", jdks.len());
    println!();

    for jdk in jdks {
        println!("  - {}@{}", jdk.distribution, jdk.version);
    }

    println!();
    println!(
        "Total disk space to be freed: {:.2} MB",
        total_disk_space as f64 / 1_048_576.0
    );
    println!();

    print!("Do you want to continue? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Display summary after successful uninstall
pub fn display_uninstall_summary(jdks: &[InstalledJdk], total_disk_space: u64) {
    if jdks.len() == 1 {
        println!(
            "Successfully uninstalled: {}@{}",
            jdks[0].distribution, jdks[0].version
        );
    } else {
        println!("Successfully uninstalled {} JDKs:", jdks.len());
        for jdk in jdks {
            println!("  - {}@{}", jdk.distribution, jdk.version);
        }
    }
    println!(
        "Disk space freed: {:.2} MB",
        total_disk_space as f64 / 1_048_576.0
    );
}

/// Display batch uninstall progress summary
pub fn display_batch_uninstall_summary(
    succeeded: &[InstalledJdk],
    failed: &[(InstalledJdk, String)],
    total_disk_space: u64,
) {
    println!();
    println!("Batch uninstall summary:");
    println!("  Succeeded: {}", succeeded.len());
    println!("  Failed: {}", failed.len());

    if !failed.is_empty() {
        println!("\nFailed uninstalls:");
        for (jdk, error) in failed {
            println!("  - {}@{}: {}", jdk.distribution, jdk.version, error);
        }
    }

    if !succeeded.is_empty() {
        println!(
            "\nDisk space freed: {:.2} MB",
            total_disk_space as f64 / 1_048_576.0
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::fixtures::create_test_jdk;

    #[test]
    fn test_display_uninstall_summary_single() {
        let jdks = vec![create_test_jdk("temurin", "21.0.5+11")];
        let disk_space = 512 * 1024 * 1024; // 512 MB

        // This test just ensures the function doesn't panic
        display_uninstall_summary(&jdks, disk_space);
    }

    #[test]
    fn test_display_uninstall_summary_multiple() {
        let jdks = vec![
            create_test_jdk("temurin", "21.0.5+11"),
            create_test_jdk("corretto", "17.0.13.11.1"),
        ];
        let disk_space = 1024 * 1024 * 1024; // 1 GB

        // This test just ensures the function doesn't panic
        display_uninstall_summary(&jdks, disk_space);
    }

    #[test]
    fn test_display_batch_uninstall_summary() {
        let succeeded = vec![
            create_test_jdk("temurin", "21.0.5+11"),
            create_test_jdk("corretto", "17.0.13.11.1"),
        ];
        let failed = vec![(
            create_test_jdk("zulu", "11.0.25"),
            "Permission denied".to_string(),
        )];
        let disk_space = 1024 * 1024 * 1024; // 1 GB

        // This test just ensures the function doesn't panic
        display_batch_uninstall_summary(&succeeded, &failed, disk_space);
    }
}
