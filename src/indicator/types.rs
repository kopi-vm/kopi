/// Configuration for a progress indicator operation
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    /// Operation name (e.g., "Downloading", "Installing", "Extracting")
    pub operation: String,

    /// Context-specific message (e.g., "temurin@21", "JDK archive")
    pub context: String,

    /// Total units for determinate operations (None for indeterminate/spinner)
    pub total: Option<u64>,

    /// Display style
    pub style: ProgressStyle,
}

impl ProgressConfig {
    /// Creates a new progress configuration
    pub fn new(
        operation: impl Into<String>,
        context: impl Into<String>,
        style: ProgressStyle,
    ) -> Self {
        Self {
            operation: operation.into(),
            context: context.into(),
            total: None,
            style,
        }
    }

    /// Sets the total for determinate operations
    pub fn with_total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }
}

/// Progress display style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    /// Progress bar with bytes display (for downloads)
    Bytes,
    /// Progress bar with count display (for batch operations)
    Count,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self::Count
    }
}

impl std::fmt::Display for ProgressStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes => write!(f, "bytes"),
            Self::Count => write!(f, "count"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_config_construction() {
        let config = ProgressConfig::new("Downloading", "temurin@21", ProgressStyle::Bytes);
        assert_eq!(config.operation, "Downloading");
        assert_eq!(config.context, "temurin@21");
        assert_eq!(config.style, ProgressStyle::Bytes);
        assert_eq!(config.total, None);
    }

    #[test]
    fn test_progress_config_with_total() {
        let config = ProgressConfig::new("Installing", "JDK", ProgressStyle::Count).with_total(100);
        assert_eq!(config.total, Some(100));
    }

    #[test]
    fn test_progress_style_default() {
        assert_eq!(ProgressStyle::default(), ProgressStyle::Count);
    }

    #[test]
    fn test_progress_style_display() {
        assert_eq!(format!("{}", ProgressStyle::Bytes), "bytes");
        assert_eq!(format!("{}", ProgressStyle::Count), "count");
    }

    #[test]
    fn test_progress_config_clone() {
        let config =
            ProgressConfig::new("Testing", "unit test", ProgressStyle::Count).with_total(50);
        let cloned = config.clone();
        assert_eq!(cloned.operation, config.operation);
        assert_eq!(cloned.context, config.context);
        assert_eq!(cloned.style, config.style);
        assert_eq!(cloned.total, config.total);
    }
}
