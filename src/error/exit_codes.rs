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

use crate::error::KopiError;

pub fn get_exit_code(error: &KopiError) -> i32 {
    match error {
        KopiError::InvalidVersionFormat(_)
        | KopiError::InvalidConfig(_)
        | KopiError::ValidationError(_) => 2,

        KopiError::NoLocalVersion { .. } => 3,

        KopiError::JdkNotInstalled { .. } => 4,

        KopiError::ToolNotFound { .. } => 5,

        KopiError::PermissionDenied(_) => 13,

        KopiError::NetworkError(_) | KopiError::Http(_) | KopiError::MetadataFetch(_) => 20,

        KopiError::DiskSpaceError(_) => 28,

        KopiError::AlreadyExists(_) => 17,

        KopiError::KopiNotFound { .. } => 127, // Standard "command not found" exit code

        KopiError::ShellDetectionError(_) => 6,
        KopiError::ShellNotFound(_) => 127, // Standard "command not found" exit code
        KopiError::UnsupportedShell(_) => 7,

        _ => 1,
    }
}
