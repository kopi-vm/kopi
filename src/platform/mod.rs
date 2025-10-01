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

//! Platform detection utilities for the entire application.
//!
//! This module provides functions to detect the current system's platform
//! characteristics (architecture, OS, libc type) which are used throughout
//! the application for platform-specific behavior.

// Re-export modules
pub mod file_ops;
pub mod filesystem;
pub mod process;
pub mod shell;
pub mod shim;
pub mod symlink;

// Internal modules
mod constants;
mod detection;

// Re-export detection functions
pub use detection::{
    get_current_architecture, get_current_os, get_current_platform, get_foojay_libc_type,
    get_platform_description, get_platform_libc, get_required_libc_type, matches_foojay_libc_type,
};

// Re-export constants
pub use constants::{
    executable_extension, is_reserved_name, kopi_binary_name, path_separator, shim_binary_name,
    uses_symlinks_for_shims, with_executable_extension,
};

pub use filesystem::{
    AdvisorySupport, DefaultFilesystemInspector, FilesystemInfo, FilesystemInspector,
    FilesystemKind,
};
