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

pub mod foojay;
pub mod generator;
pub mod generator_config;
pub mod http;
pub mod index;
pub mod local;
pub mod provider;
pub mod source;

pub use foojay::FoojayMetadataSource;
pub use generator::{GeneratorConfig, MetadataGenerator, Platform};
pub use generator_config::MetadataGenConfigFile;
pub use http::HttpMetadataSource;
pub use index::{IndexFile, IndexFileEntry};
pub use local::LocalDirectorySource;
pub use provider::{MetadataProvider, SourceHealth};
pub use source::{MetadataSource, PackageDetails};
