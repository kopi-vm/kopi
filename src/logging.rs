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
