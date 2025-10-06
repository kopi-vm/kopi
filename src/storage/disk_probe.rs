use crate::error::{KopiError, Result};
use log::debug;
use std::path::{Path, PathBuf};
use sysinfo::{Disk, Disks};

/// Return the available bytes for the filesystem containing `path`.
pub fn available_bytes(path: &Path) -> Result<u64> {
    let resolved_path = canonicalize_or_clone(path);
    let normalized_target = normalize_for_matching(&resolved_path);

    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh();

    let disk_list = disks.list();
    let Some(disk) = find_best_disk(normalized_target.as_path(), disk_list) else {
        return Err(KopiError::SystemError(format!(
            "Unable to determine mount point for {}",
            resolved_path.display()
        )));
    };

    Ok(DiskRecord::available_space(disk))
}

fn canonicalize_or_clone(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(err) => {
            debug!(
                "Falling back to non-canonical path {} due to error: {}",
                path.display(),
                err
            );
            path.to_path_buf()
        }
    }
}

fn normalize_for_matching(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::borrow::Cow;
        let text: Cow<'_, str> = path.to_string_lossy();
        let trimmed = text.trim_start_matches(r"\\?\");
        PathBuf::from(trimmed)
    }

    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

fn find_best_disk<'a, D>(target: &Path, disks: &'a [D]) -> Option<&'a D>
where
    D: DiskRecord + 'a,
{
    let mut best_match: Option<&D> = None;
    let mut best_depth: usize = 0;

    for disk in disks {
        let normalized_mount = normalize_for_matching(disk.mount_point());
        if target.starts_with(&normalized_mount) {
            let depth = normalized_mount.components().count();
            if depth >= best_depth {
                best_depth = depth;
                best_match = Some(disk);
            }
        }
    }

    best_match
}

trait DiskRecord {
    fn mount_point(&self) -> &Path;
    fn available_space(&self) -> u64;
}

impl DiskRecord for Disk {
    fn mount_point(&self) -> &Path {
        Disk::mount_point(self)
    }

    fn available_space(&self) -> u64 {
        Disk::available_space(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockDisk {
        mount: PathBuf,
        available: u64,
    }

    impl MockDisk {
        fn new(mount: &str, available: u64) -> Self {
            Self {
                mount: PathBuf::from(mount),
                available,
            }
        }
    }

    impl DiskRecord for MockDisk {
        fn mount_point(&self) -> &Path {
            &self.mount
        }

        fn available_space(&self) -> u64 {
            self.available
        }
    }

    #[test]
    fn selects_deepest_mount_on_unix() {
        let target = Path::new("/Users/demo/project");
        let normalized_target = normalize_for_matching(target);
        let disks = vec![
            MockDisk::new("/", 10 * 1024 * 1024),
            MockDisk::new("/Users", 20 * 1024 * 1024),
        ];

        let selected = find_best_disk(normalized_target.as_path(), &disks);
        assert_eq!(
            DiskRecord::available_space(selected.unwrap()),
            20 * 1024 * 1024
        );
    }

    #[test]
    fn falls_back_to_root_mount() {
        let target = Path::new("/var/data");
        let normalized_target = normalize_for_matching(target);
        let disks = vec![MockDisk::new("/", 42)];

        let selected = find_best_disk(normalized_target.as_path(), &disks);
        assert_eq!(DiskRecord::available_space(selected.unwrap()), 42);
    }

    #[cfg(windows)]
    #[test]
    fn normalizes_extended_windows_paths() {
        let target = Path::new(r"\\?\C:\\Users\\demo\\project");
        let normalized_target = normalize_for_matching(target);
        assert_eq!(
            normalized_target.to_str().unwrap(),
            r"C:\\Users\\demo\\project"
        );

        let disks = vec![MockDisk::new(r"C:\", 100), MockDisk::new(r"D:\", 200)];

        let selected = find_best_disk(normalized_target.as_path(), &disks);
        assert_eq!(DiskRecord::available_space(selected.unwrap()), 100);
    }

    #[cfg(not(windows))]
    #[test]
    fn ignores_windows_specific_normalization_on_unix() {
        let target = Path::new("/home/demo");
        let normalized_target = normalize_for_matching(target);
        assert_eq!(normalized_target, PathBuf::from("/home/demo"));
    }
}
