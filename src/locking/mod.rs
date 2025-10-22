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

pub mod controller;
pub mod fallback;
pub mod handle;
pub mod hygiene;
pub mod package_coordinate;
pub mod scope;
pub mod timeout;

pub use controller::{LockAcquisition, LockController};
pub use handle::{FallbackHandle, LockBackend, LockHandle};
pub use hygiene::{LockHygieneReport, LockHygieneRunner, run_startup_hygiene};
pub use package_coordinate::{PackageCoordinate, PackageKind};
pub use scope::{LockKind, LockScope};
pub use timeout::{
    LockTimeoutResolution, LockTimeoutResolver, LockTimeoutSource, LockTimeoutValue,
    parse_timeout_override,
};
