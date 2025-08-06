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

pub mod cache;
pub mod installation;
pub mod jdks;
pub mod network;
pub mod permissions;
pub mod shell;

pub use cache::{
    CacheFileCheck, CacheFormatCheck, CachePermissionsCheck, CacheSizeCheck, CacheStalenessCheck,
};
pub use installation::{
    ConfigFileCheck, InstallationDirectoryCheck, KopiBinaryCheck, ShimsInPathCheck, VersionCheck,
};
pub use jdks::{
    JdkDiskSpaceCheck, JdkInstallationCheck, JdkIntegrityCheck, JdkVersionConsistencyCheck,
};
pub use network::{
    ApiConnectivityCheck, DnsResolutionCheck, ProxyConfigurationCheck, TlsVerificationCheck,
};
pub use permissions::{BinaryPermissionsCheck, DirectoryPermissionsCheck};
pub use shell::{PathCheck, ShellConfigurationCheck, ShellDetectionCheck, ShimFunctionalityCheck};
