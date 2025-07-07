use env_logger;

/// Initialize the logger with the specified verbosity level
///
/// # Arguments
/// * `verbose` - Verbosity level (0=warn, 1=info, 2=debug, 3+=trace)
pub fn setup_logger(verbose: u8) {
    let env_filter = match verbose {
        0 => "kopi=warn",
        1 => "kopi=info",
        2 => "kopi=debug",
        _ => "kopi=trace",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(env_filter))
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();
}
