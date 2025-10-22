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

use log::warn;
use signal_hook::SigId;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::flag;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

#[cfg(windows)]
use signal_hook::consts::signal::SIGBREAK;

/// Token used to observe cancellation signals triggered by the user.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn from_shared(flag: Arc<AtomicBool>) -> Self {
        Self { flag }
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// Marks the token as cancelled. Intended for internal use and tests.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct CancellationRegistry {
    flag: Arc<AtomicBool>,
    _handles: Vec<SigId>,
}

impl CancellationRegistry {
    fn new() -> Self {
        let flag = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();

        for signal in registered_signals() {
            match flag::register(*signal, flag.clone()) {
                Ok(handle) => handles.push(handle),
                Err(err) => {
                    warn!("Failed to register cancellation handler for signal {signal}: {err}")
                }
            }
        }

        Self {
            flag,
            _handles: handles,
        }
    }

    fn token(&self) -> CancellationToken {
        CancellationToken::from_shared(self.flag.clone())
    }
}

fn registered_signals() -> &'static [i32] {
    #[cfg(windows)]
    {
        static SIGNALS: [i32; 3] = [SIGINT, SIGTERM, SIGBREAK];
        &SIGNALS
    }

    #[cfg(not(windows))]
    {
        static SIGNALS: [i32; 2] = [SIGINT, SIGTERM];
        &SIGNALS
    }
}

static GLOBAL_REGISTRY: OnceLock<CancellationRegistry> = OnceLock::new();

/// Returns a cancellation token backed by global signal handlers.
pub fn global_token() -> CancellationToken {
    GLOBAL_REGISTRY
        .get_or_init(CancellationRegistry::new)
        .token()
}
