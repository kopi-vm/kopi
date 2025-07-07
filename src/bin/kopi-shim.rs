use kopi::shim;
use std::env;
use std::process;

fn main() {
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
