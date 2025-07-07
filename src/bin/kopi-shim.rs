use kopi::{logging, shim};
use std::env;
use std::process;

fn main() {
    // Initialize logger with default verbosity (warn level)
    // This will respect RUST_LOG environment variable if set
    logging::setup_logger(0);

    // Get the tool name from argv[0]
    let args: Vec<String> = env::args().collect();

    // Run the shim runtime
    match shim::run(args) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}
