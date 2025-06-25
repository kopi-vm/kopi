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
