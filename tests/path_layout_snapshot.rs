use kopi::paths::{cache, home, install, locking, shims};
use tempfile::TempDir;

#[test]
fn path_layout_snapshot_matches_expected_structure() {
    let temp = TempDir::new().unwrap();
    let kopi_home = temp.path();

    assert_eq!(home::jdks_dir(kopi_home), kopi_home.join("jdks"));
    assert_eq!(home::cache_dir(kopi_home), kopi_home.join("cache"));
    assert_eq!(home::shims_dir(kopi_home), kopi_home.join("shims"));
    assert_eq!(home::bin_dir(kopi_home), kopi_home.join("bin"));
    assert_eq!(home::locks_dir(kopi_home), kopi_home.join("locks"));

    let slug = "temurin-21-jdk-x64";
    assert_eq!(
        install::installation_directory(kopi_home, slug),
        kopi_home.join("jdks").join(slug)
    );
    assert_eq!(
        install::metadata_file(kopi_home, slug),
        kopi_home.join("jdks").join(format!("{slug}.meta.json"))
    );
    assert_eq!(
        install::temp_staging_directory(kopi_home),
        kopi_home.join("jdks").join(".tmp")
    );

    assert_eq!(
        cache::metadata_cache_file(kopi_home),
        kopi_home.join("cache").join("metadata.json")
    );
    assert_eq!(
        cache::temp_cache_directory(kopi_home),
        kopi_home.join("cache").join("tmp")
    );

    assert_eq!(shims::shims_root(kopi_home), kopi_home.join("shims"));
    let shim_binary = kopi::platform::shim_binary_name();
    assert_eq!(
        shims::shim_launcher_path(kopi_home),
        kopi_home.join("shims").join(shim_binary)
    );
    assert_eq!(
        shims::tool_shim_path(kopi_home, "java"),
        kopi_home
            .join("shims")
            .join(kopi::platform::with_executable_extension("java"))
    );

    assert_eq!(locking::locks_root(kopi_home), kopi_home.join("locks"));
    assert_eq!(
        locking::install_lock_directory(kopi_home, "Temurin FX"),
        kopi_home.join("locks").join("install").join("temurin-fx")
    );

    // Ensure helper-backed directory creation mirrors expected layout.
    assert!(
        install::ensure_installations_root(kopi_home)
            .unwrap()
            .exists()
    );
    assert!(cache::ensure_cache_root(kopi_home).unwrap().exists());
    assert!(shims::ensure_shims_root(kopi_home).unwrap().exists());
    assert!(home::ensure_bin_dir(kopi_home).unwrap().exists());
    assert!(home::ensure_locks_dir(kopi_home).unwrap().exists());
}
