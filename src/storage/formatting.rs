/// Formats a byte size into a human-readable string with appropriate units.
///
/// Uses binary units (1024) for conversion and displays one decimal place
/// for all units except bytes.
///
/// # Arguments
/// * `bytes` - The size in bytes to format
///
/// # Returns
/// A formatted string with the size and appropriate unit (B, KB, MB, GB, TB)
///
/// # Examples
/// ```
/// use kopi::storage::formatting::format_size;
///
/// assert_eq!(format_size(512), "512 B");
/// assert_eq!(format_size(1536), "1.5 KB");
/// assert_eq!(format_size(1048576), "1.0 MB");
/// ```
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048575), "1024.0 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1572864), "1.5 MB");
        assert_eq!(format_size(1073741823), "1024.0 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(1610612736), "1.5 GB");
    }

    #[test]
    fn test_format_size_terabytes() {
        assert_eq!(format_size(1099511627776), "1.0 TB");
        assert_eq!(format_size(1649267441664), "1.5 TB");
    }
}
