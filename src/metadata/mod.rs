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
pub use provider::MetadataProvider;
pub use source::{MetadataSource, PackageDetails};
