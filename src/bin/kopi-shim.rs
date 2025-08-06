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
