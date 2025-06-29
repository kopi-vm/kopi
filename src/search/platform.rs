//! Platform detection utilities for search filtering.
//!
//! This module provides functions to detect the current system's platform
//! characteristics (architecture, OS, libc type) which are used for
//! automatic platform filtering during searches.

use std::sync::OnceLock;

/// Cached platform information to avoid repeated system calls.
/// Stores (architecture, operating_system, lib_c_type) tuple.
static CACHED_PLATFORM: OnceLock<(String, String, String)> = OnceLock::new();

/// Get current platform information with caching.
///
/// Returns a tuple of (architecture, operating_system, lib_c_type) for the
/// current system. The result is cached on first call to avoid repeated
/// system detection.
///
/// # Returns
///
/// A tuple containing:
/// - Architecture string (e.g., "x64", "aarch64")
/// - Operating system string (e.g., "linux", "windows", "macos")
/// - libc type string (e.g., "glibc", "musl")
///
/// # Example
///
/// ```
/// use kopi::search::get_current_platform;
///
/// let (arch, os, libc) = get_current_platform();
/// println!("Running on {} {} with {}", arch, os, libc);
/// ```
pub fn get_current_platform() -> (String, String, String) {
    CACHED_PLATFORM
        .get_or_init(|| {
            let arch = get_current_architecture();
            let os = get_current_os();
            let lib_c_type = crate::platform::get_foojay_libc_type();
            (arch, os, lib_c_type.to_string())
        })
        .clone()
}

/// Detect the current system architecture.
///
/// Maps Rust's target architecture to foojay.io's architecture naming:
/// - `x86_64` → `"x64"`
/// - `x86` → `"x86"`
/// - `aarch64` → `"aarch64"`
/// - `arm` → `"arm32"`
/// - `powerpc64` → `"ppc64le"` (little endian) or `"ppc64"` (big endian)
/// - `s390x` → `"s390x"`
/// - Others → `"unknown"`
pub fn get_current_architecture() -> String {
    #[cfg(target_arch = "x86_64")]
    return "x64".to_string();

    #[cfg(target_arch = "x86")]
    return "x86".to_string();

    #[cfg(target_arch = "aarch64")]
    return "aarch64".to_string();

    #[cfg(target_arch = "arm")]
    return "arm32".to_string();

    #[cfg(target_arch = "powerpc64")]
    {
        #[cfg(target_endian = "little")]
        return "ppc64le".to_string();
        #[cfg(target_endian = "big")]
        return "ppc64".to_string();
    }

    #[cfg(target_arch = "s390x")]
    return "s390x".to_string();

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc64",
        target_arch = "s390x"
    )))]
    return "unknown".to_string();
}

/// Detect the current operating system.
///
/// Maps Rust's target OS to foojay.io's OS naming:
/// - `linux` → `"linux"`
/// - `windows` → `"windows"`
/// - `macos` → `"macos"`
/// - Others → `"unknown"`
pub fn get_current_os() -> String {
    #[cfg(target_os = "linux")]
    return "linux".to_string();

    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "macos")]
    return "macos".to_string();

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return "unknown".to_string();
}
