// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Filesystem inspection utilities used by the locking subsystem.
//!
//! The inspector provides a lightweight abstraction over platform-specific
//! filesystem queries. It reports whether advisory locks are expected to work
//! reliably on a given mount and flags network shares so the caller can fall
//! back to safer strategies when needed.

use crate::error::{KopiError, Result};
use std::path::{Path, PathBuf};

/// Indicates whether native advisory locks should be used on the target filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisorySupport {
    /// Advisory locks are expected to function reliably.
    Native,
    /// Advisory locks are known to be unreliable; callers should fall back to
    /// atomic rename flows.
    RequiresFallback,
    /// Capability is unknown; the caller may attempt advisory locks and cache
    /// the outcome.
    Unknown,
}

/// Classifies filesystems relevant to Kopi's locking subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesystemKind {
    Ext4,
    Xfs,
    Btrfs,
    Apfs,
    Ntfs,
    Tmpfs,
    Overlay,
    Zfs,
    Fat,
    Exfat,
    Nfs,
    Cifs,
    Smb2,
    Other(String),
}

impl FilesystemKind {
    fn other_from_identifier(identifier: impl Into<String>) -> Self {
        FilesystemKind::Other(identifier.into())
    }
}

/// Summary of filesystem characteristics relevant to locking decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemInfo {
    pub kind: FilesystemKind,
    pub advisory_support: AdvisorySupport,
    pub is_network_share: bool,
}

impl FilesystemInfo {
    fn new(
        kind: FilesystemKind,
        advisory_support: AdvisorySupport,
        is_network_share: bool,
    ) -> Self {
        Self {
            kind,
            advisory_support,
            is_network_share,
        }
    }

    fn unknown(identifier: impl Into<String>) -> Self {
        Self::new(
            FilesystemKind::other_from_identifier(identifier),
            AdvisorySupport::Unknown,
            false,
        )
    }
}

/// Abstract interface for filesystem inspectors.
pub trait FilesystemInspector: Send + Sync {
    /// Classifies the filesystem backing `path`.
    fn classify(&self, path: &Path) -> Result<FilesystemInfo>;
}

/// Default filesystem inspector that performs live OS queries.
#[derive(Debug, Default)]
pub struct DefaultFilesystemInspector;

impl DefaultFilesystemInspector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl FilesystemInspector for DefaultFilesystemInspector {
    fn classify(&self, path: &Path) -> Result<FilesystemInfo> {
        let probe_target = resolve_probe_target(path)?;

        #[cfg(unix)]
        {
            classify_unix(&probe_target)
        }

        #[cfg(windows)]
        {
            classify_windows(&probe_target)
        }

        #[cfg(not(any(unix, windows)))]
        {
            let identifier = probe_target.display().to_string();
            Ok(FilesystemInfo::unknown(identifier))
        }
    }
}

fn resolve_probe_target(original: &Path) -> Result<PathBuf> {
    if let Ok(existing) = original.canonicalize() {
        return Ok(existing);
    }

    if let Some(parent) = original.parent()
        && let Ok(existing) = parent.canonicalize()
    {
        return Ok(existing);
    }

    std::env::current_dir().map_err(|e| {
        KopiError::SystemError(format!(
            "Failed to resolve filesystem for '{}': {e}",
            original.display()
        ))
    })
}

#[cfg(unix)]
fn classify_unix(path: &Path) -> Result<FilesystemInfo> {
    use nix::sys::statfs::statfs;

    let stats = statfs(path).map_err(|err| {
        KopiError::SystemError(format!(
            "Failed to query filesystem for '{}': {err}",
            path.display()
        ))
    })?;

    Ok(describe_unix_filesystem(&stats))
}

#[cfg(unix)]
fn describe_unix_filesystem(stats: &nix::sys::statfs::Statfs) -> FilesystemInfo {
    use nix::sys::statfs::FsType;

    let fs_type: FsType = stats.filesystem_type();
    let raw = fs_type.0 as libc::c_long;
    if let Some(info) = classify_unix_magic(raw) {
        return info;
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    {
        if let Some(name) = stats.fstypename() {
            if let Ok(name_str) = name.to_str() {
                return classify_by_name(name_str, raw);
            }
        }
    }

    FilesystemInfo::unknown(format!("0x{raw:x}"))
}

#[cfg(unix)]
fn classify_unix_magic(raw: libc::c_long) -> Option<FilesystemInfo> {
    match raw {
        EXT4_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Ext4,
            AdvisorySupport::Native,
            false,
        )),
        XFS_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Xfs,
            AdvisorySupport::Native,
            false,
        )),
        BTRFS_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Btrfs,
            AdvisorySupport::Native,
            false,
        )),
        TMPFS_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Tmpfs,
            AdvisorySupport::Unknown,
            false,
        )),
        OVERLAYFS_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Overlay,
            AdvisorySupport::Unknown,
            false,
        )),
        ZFS_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Zfs,
            AdvisorySupport::Native,
            false,
        )),
        CIFS_MAGIC_NUMBER => Some(FilesystemInfo::new(
            FilesystemKind::Cifs,
            AdvisorySupport::RequiresFallback,
            true,
        )),
        SMB2_MAGIC_NUMBER => Some(FilesystemInfo::new(
            FilesystemKind::Smb2,
            AdvisorySupport::RequiresFallback,
            true,
        )),
        NFS_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Nfs,
            AdvisorySupport::RequiresFallback,
            true,
        )),
        MSDOS_SUPER_MAGIC | VFAT_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Fat,
            AdvisorySupport::RequiresFallback,
            false,
        )),
        EXFAT_SUPER_MAGIC => Some(FilesystemInfo::new(
            FilesystemKind::Exfat,
            AdvisorySupport::RequiresFallback,
            false,
        )),
        _ => None,
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn classify_by_name(name: &str, fallback_raw: libc::c_long) -> FilesystemInfo {
    let normalized = name.to_ascii_lowercase();
    match normalized.as_str() {
        "apfs" => FilesystemInfo::new(FilesystemKind::Apfs, AdvisorySupport::Native, false),
        "ntfs" => FilesystemInfo::new(FilesystemKind::Ntfs, AdvisorySupport::Native, false),
        "zfs" => FilesystemInfo::new(FilesystemKind::Zfs, AdvisorySupport::Native, false),
        "hfs" | "hfs+" => FilesystemInfo::new(
            FilesystemKind::Other("hfs".to_string()),
            AdvisorySupport::Unknown,
            false,
        ),
        "nfs" => FilesystemInfo::new(FilesystemKind::Nfs, AdvisorySupport::RequiresFallback, true),
        "smbfs" | "cifs" => FilesystemInfo::new(
            FilesystemKind::Cifs,
            AdvisorySupport::RequiresFallback,
            true,
        ),
        other_name => FilesystemInfo::unknown(format!("{other_name} (0x{fallback_raw:x})")),
    }
}

#[cfg(unix)]
const EXT4_SUPER_MAGIC: libc::c_long = 0xEF53;
#[cfg(unix)]
const XFS_SUPER_MAGIC: libc::c_long = 0x5846_5342;
#[cfg(unix)]
const BTRFS_SUPER_MAGIC: libc::c_long = 0x9123_683E;
#[cfg(unix)]
const TMPFS_MAGIC: libc::c_long = 0x0102_1994;
#[cfg(unix)]
const OVERLAYFS_SUPER_MAGIC: libc::c_long = 0x794C_7630;
#[cfg(unix)]
const ZFS_SUPER_MAGIC: libc::c_long = 0x2FC1_2FC1;
#[cfg(unix)]
const CIFS_MAGIC_NUMBER: libc::c_long = 0xFF53_4D42;
#[cfg(unix)]
const SMB2_MAGIC_NUMBER: libc::c_long = 0xFE53_4D42;
#[cfg(unix)]
const NFS_SUPER_MAGIC: libc::c_long = 0x0000_6969;
#[cfg(unix)]
const MSDOS_SUPER_MAGIC: libc::c_long = 0x0000_4D44;
#[cfg(unix)]
const VFAT_SUPER_MAGIC: libc::c_long = 0x0000_5646;
#[cfg(unix)]
const EXFAT_SUPER_MAGIC: libc::c_long = 0x2011_BAB0;

#[cfg(windows)]
fn classify_windows(path: &Path) -> Result<FilesystemInfo> {
    use std::convert::TryInto;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;
    use winapi::shared::minwindef::{DWORD, MAX_PATH};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::fileapi::{GetDriveTypeW, GetVolumeInformationW, GetVolumePathNameW};
    use winapi::um::winbase::{DRIVE_REMOTE, DRIVE_UNKNOWN};

    let wide_path: Vec<u16> = ensure_leading_root(path)
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();

    const MAX_PATH_LEN: usize = MAX_PATH;
    let mut volume_path = [0u16; MAX_PATH_LEN];
    let buffer_capacity: DWORD = MAX_PATH_LEN
        .try_into()
        .expect("MAX_PATH exceeds DWORD range");
    let ok = unsafe {
        GetVolumePathNameW(
            wide_path.as_ptr(),
            volume_path.as_mut_ptr(),
            buffer_capacity,
        )
    };
    if ok == 0 {
        return Err(KopiError::SystemError(format!(
            "Failed to resolve volume for '{}': Win32 error {}",
            path.display(),
            unsafe { GetLastError() }
        )));
    }

    let drive_type = unsafe { GetDriveTypeW(volume_path.as_ptr()) };
    let is_network_share = drive_type == DRIVE_REMOTE;

    let mut fs_name_buffer = [0u16; MAX_PATH_LEN];
    let mut serial: DWORD = 0;
    let mut max_component_len: DWORD = 0;
    let mut fs_flags: DWORD = 0;

    let ok = unsafe {
        GetVolumeInformationW(
            volume_path.as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            &mut max_component_len,
            &mut fs_flags,
            fs_name_buffer.as_mut_ptr(),
            buffer_capacity,
        )
    };

    if ok == 0 {
        return Err(KopiError::SystemError(format!(
            "Failed to query filesystem for '{}': Win32 error {}",
            path.display(),
            unsafe { GetLastError() }
        )));
    }

    let fs_name = wide_to_string(&fs_name_buffer);
    Ok(classify_windows_by_name(
        &fs_name,
        is_network_share,
        drive_type == DRIVE_UNKNOWN,
    ))
}

#[cfg(windows)]
fn ensure_leading_root(path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_path_buf();
    }

    if let Some(parent) = path.parent()
        && parent.exists()
    {
        return parent.to_path_buf();
    }

    path.to_path_buf()
}

#[cfg(windows)]
fn wide_to_string(buffer: &[u16]) -> String {
    let terminator = buffer
        .iter()
        .position(|&ch| ch == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..terminator])
}

#[cfg(windows)]
fn classify_windows_by_name(
    name: &str,
    is_network_share: bool,
    drive_unknown: bool,
) -> FilesystemInfo {
    let normalized = name.to_ascii_lowercase();
    match normalized.as_str() {
        "ntfs" => FilesystemInfo::new(
            FilesystemKind::Ntfs,
            AdvisorySupport::Native,
            is_network_share,
        ),
        "refs" => FilesystemInfo::new(
            FilesystemKind::Other("refs".to_string()),
            AdvisorySupport::Native,
            is_network_share,
        ),
        "fat" | "fat32" | "fat16" => FilesystemInfo::new(
            FilesystemKind::Fat,
            AdvisorySupport::RequiresFallback,
            is_network_share,
        ),
        "exfat" => FilesystemInfo::new(
            FilesystemKind::Exfat,
            AdvisorySupport::RequiresFallback,
            is_network_share,
        ),
        "cifs" | "smb" | "smb2" => FilesystemInfo::new(
            FilesystemKind::Smb2,
            AdvisorySupport::RequiresFallback,
            true,
        ),
        other if drive_unknown => FilesystemInfo::unknown(other.to_string()),
        other => FilesystemInfo::unknown(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn unix_classifies_known_filesystems() {
        let ext4 = classify_unix_magic(EXT4_SUPER_MAGIC).unwrap();
        assert_eq!(ext4.kind, FilesystemKind::Ext4);
        assert_eq!(ext4.advisory_support, AdvisorySupport::Native);
        assert!(!ext4.is_network_share);

        let cifs = classify_unix_magic(CIFS_MAGIC_NUMBER).unwrap();
        assert_eq!(cifs.kind, FilesystemKind::Cifs);
        assert_eq!(cifs.advisory_support, AdvisorySupport::RequiresFallback);
        assert!(cifs.is_network_share);
    }

    #[test]
    fn filesystems_have_displayable_unknown_default() {
        let info = FilesystemInfo::unknown("example".to_string());
        assert_eq!(info.advisory_support, AdvisorySupport::Unknown);
        match info.kind {
            FilesystemKind::Other(name) => assert_eq!(name, "example"),
            _ => panic!("expected other kind"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_classifies_ntfs() {
        let info = classify_windows_by_name("NTFS", false, false);
        assert_eq!(info.kind, FilesystemKind::Ntfs);
        assert_eq!(info.advisory_support, AdvisorySupport::Native);
        assert!(!info.is_network_share);
    }
}
