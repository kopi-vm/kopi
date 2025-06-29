//! Platform detection utilities for the entire application.
//!
//! This module provides functions to detect the current system's platform
//! characteristics (architecture, OS, libc type) which are used throughout
//! the application for platform-specific behavior.

use std::sync::OnceLock;

// Platform-specific libc detection
#[cfg(all(target_os = "linux", target_env = "musl"))]
const PLATFORM_LIBC: &str = "musl";

#[cfg(all(target_os = "linux", target_env = "gnu"))]
const PLATFORM_LIBC: &str = "glibc";

#[cfg(target_os = "macos")]
const PLATFORM_LIBC: &str = "darwin"; // macOS uses its own system libraries

#[cfg(target_os = "windows")]
const PLATFORM_LIBC: &str = "windows"; // Windows uses MSVCRT

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const PLATFORM_LIBC: &str = "unknown";

/// Cached platform information to avoid repeated system calls.
/// Stores (architecture, operating_system, lib_c_type) tuple.
static CACHED_PLATFORM: OnceLock<(String, String, String)> = OnceLock::new();

/// Get the platform libc type for debugging and informational purposes
pub fn get_platform_libc() -> &'static str {
    PLATFORM_LIBC
}

/// Get Foojay API lib_c_type value for current platform
pub fn get_foojay_libc_type() -> &'static str {
    match PLATFORM_LIBC {
        "musl" => "musl",
        "glibc" => "glibc",       // Return "glibc" for glibc systems
        "darwin" => "libc",       // macOS uses "libc" in Foojay API
        "windows" => "c_std_lib", // Windows uses "c_std_lib" in Foojay API
        _ => "libc",              // Default fallback
    }
}

/// Match against Foojay API lib_c_type values
pub fn matches_foojay_libc_type(foojay_libc: &str) -> bool {
    match (PLATFORM_LIBC, foojay_libc) {
        ("musl", "musl") => true,
        ("glibc", "libc") | ("glibc", "glibc") => true,
        ("darwin", "libc") => true, // macOS uses "libc" in Foojay API
        ("windows", "c_std_lib") => true, // Windows uses "c_std_lib" in Foojay API
        _ => false,
    }
}

/// Get the required libc type for Foojay API queries
pub fn get_required_libc_type() -> &'static str {
    get_foojay_libc_type()
}

/// Get a user-friendly description of the current platform
pub fn get_platform_description() -> String {
    match PLATFORM_LIBC {
        "musl" => "Alpine Linux (musl)".to_string(),
        "glibc" => "Linux (glibc)".to_string(),
        "darwin" => "macOS".to_string(),
        "windows" => "Windows".to_string(),
        _ => "Unknown platform".to_string(),
    }
}

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
/// use kopi::platform::get_current_platform;
///
/// let (arch, os, libc) = get_current_platform();
/// println!("Running on {} {} with {}", arch, os, libc);
/// ```
pub fn get_current_platform() -> (String, String, String) {
    CACHED_PLATFORM
        .get_or_init(|| {
            let arch = get_current_architecture();
            let os = get_current_os();
            let lib_c_type = get_foojay_libc_type();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_libc_constant() {
        // Test that PLATFORM_LIBC is set to a valid value
        assert!(["musl", "glibc", "darwin", "windows", "unknown"].contains(&PLATFORM_LIBC));
    }

    #[test]
    fn test_get_foojay_libc_type() {
        let libc_type = get_foojay_libc_type();
        // Test that the function returns a valid Foojay API value
        assert!(["musl", "glibc", "libc", "c_std_lib"].contains(&libc_type));
    }

    #[test]
    fn test_matches_foojay_libc_type() {
        // Test matching logic based on current platform
        match PLATFORM_LIBC {
            "musl" => {
                assert!(matches_foojay_libc_type("musl"));
                assert!(!matches_foojay_libc_type("libc"));
            }
            "glibc" => {
                assert!(matches_foojay_libc_type("libc"));
                assert!(matches_foojay_libc_type("glibc"));
                assert!(!matches_foojay_libc_type("musl"));
            }
            "darwin" => {
                assert!(matches_foojay_libc_type("libc"));
                assert!(!matches_foojay_libc_type("musl"));
            }
            "windows" => {
                assert!(matches_foojay_libc_type("c_std_lib"));
                assert!(!matches_foojay_libc_type("libc"));
            }
            _ => {}
        }
    }

    #[test]
    fn test_platform_description() {
        let description = get_platform_description();
        assert!(!description.is_empty());
    }
}
