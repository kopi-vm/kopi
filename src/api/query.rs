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

#[derive(Debug, Clone, Default)]
pub struct PackageQuery {
    pub version: Option<String>,
    pub distribution: Option<String>,
    pub architecture: Option<String>,
    pub package_type: Option<String>,
    pub operating_system: Option<String>,
    pub archive_types: Option<Vec<String>>,
    pub latest: Option<String>,
    pub directly_downloadable: Option<bool>,
    pub lib_c_type: Option<String>,
    pub javafx_bundled: Option<bool>,
}

impl PackageQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn distribution(mut self, distribution: impl Into<String>) -> Self {
        self.distribution = Some(distribution.into());
        self
    }

    pub fn architecture(mut self, architecture: impl Into<String>) -> Self {
        self.architecture = Some(architecture.into());
        self
    }

    pub fn package_type(mut self, package_type: impl Into<String>) -> Self {
        self.package_type = Some(package_type.into());
        self
    }

    pub fn operating_system(mut self, operating_system: impl Into<String>) -> Self {
        self.operating_system = Some(operating_system.into());
        self
    }

    pub fn archive_types(mut self, archive_types: Vec<String>) -> Self {
        self.archive_types = Some(archive_types);
        self
    }

    pub fn latest(mut self, latest: impl Into<String>) -> Self {
        self.latest = Some(latest.into());
        self
    }

    pub fn directly_downloadable(mut self, directly_downloadable: bool) -> Self {
        self.directly_downloadable = Some(directly_downloadable);
        self
    }

    pub fn lib_c_type(mut self, lib_c_type: impl Into<String>) -> Self {
        self.lib_c_type = Some(lib_c_type.into());
        self
    }

    pub fn javafx_bundled(mut self, javafx_bundled: bool) -> Self {
        self.javafx_bundled = Some(javafx_bundled);
        self
    }
}
