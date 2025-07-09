//! Platform-specific constants and utility functions.

/// Platform-specific path separator
pub fn path_separator() -> char {
    #[cfg(windows)]
    return ';';
    #[cfg(not(windows))]
    return ':';
}

/// Get the executable file extension for the current platform
pub fn executable_extension() -> &'static str {
    #[cfg(windows)]
    return ".exe";
    #[cfg(not(windows))]
    return "";
}

/// Add the platform-specific executable extension to a file name
pub fn with_executable_extension(name: &str) -> String {
    format!("{}{}", name, executable_extension())
}

/// Get the shim binary name for the current platform
pub fn shim_binary_name() -> &'static str {
    #[cfg(windows)]
    return "kopi-shim.exe";
    #[cfg(not(windows))]
    return "kopi-shim";
}

/// Get the kopi binary name for the current platform
pub fn kopi_binary_name() -> &'static str {
    #[cfg(windows)]
    return "kopi.exe";
    #[cfg(not(windows))]
    return "kopi";
}

/// Check if the current platform uses symlinks for shims
pub fn uses_symlinks_for_shims() -> bool {
    #[cfg(unix)]
    return true;
    #[cfg(windows)]
    return false;
}

/// Check for Windows reserved file names
#[cfg(windows)]
pub fn is_reserved_name(name: &str) -> bool {
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let upper_name = name.to_uppercase();
    RESERVED_NAMES.contains(&upper_name.as_str())
}

#[cfg(not(windows))]
pub fn is_reserved_name(_name: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_separator() {
        let sep = path_separator();
        #[cfg(windows)]
        assert_eq!(sep, ';');
        #[cfg(not(windows))]
        assert_eq!(sep, ':');
    }

    #[test]
    fn test_executable_extension() {
        let ext = executable_extension();
        #[cfg(windows)]
        assert_eq!(ext, ".exe");
        #[cfg(not(windows))]
        assert_eq!(ext, "");
    }

    #[test]
    fn test_with_executable_extension() {
        let java_exe = with_executable_extension("java");
        #[cfg(windows)]
        assert_eq!(java_exe, "java.exe");
        #[cfg(not(windows))]
        assert_eq!(java_exe, "java");

        let javac_exe = with_executable_extension("javac");
        #[cfg(windows)]
        assert_eq!(javac_exe, "javac.exe");
        #[cfg(not(windows))]
        assert_eq!(javac_exe, "javac");
    }

    #[test]
    fn test_shim_binary_name() {
        let name = shim_binary_name();
        #[cfg(windows)]
        assert_eq!(name, "kopi-shim.exe");
        #[cfg(not(windows))]
        assert_eq!(name, "kopi-shim");
    }

    #[test]
    fn test_kopi_binary_name() {
        let name = kopi_binary_name();
        #[cfg(windows)]
        assert_eq!(name, "kopi.exe");
        #[cfg(not(windows))]
        assert_eq!(name, "kopi");
    }

    #[test]
    fn test_uses_symlinks_for_shims() {
        let uses_symlinks = uses_symlinks_for_shims();
        #[cfg(unix)]
        assert!(uses_symlinks);
        #[cfg(windows)]
        assert!(!uses_symlinks);
    }

    #[cfg(windows)]
    #[test]
    fn test_reserved_names_windows() {
        assert!(is_reserved_name("CON"));
        assert!(is_reserved_name("con")); // Case insensitive
        assert!(is_reserved_name("PRN"));
        assert!(is_reserved_name("NUL"));
        assert!(!is_reserved_name("CONSOLE"));
        assert!(!is_reserved_name("java"));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_reserved_names_non_windows() {
        assert!(!is_reserved_name("CON"));
        assert!(!is_reserved_name("java"));
    }
}
