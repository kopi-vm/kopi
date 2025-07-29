pub mod foojay;
pub mod generator;
pub mod generator_config;
pub mod index;
pub mod provider;
pub mod source;

pub use foojay::FoojayMetadataSource;
pub use generator::{GeneratorConfig, MetadataGenerator, Platform};
pub use generator_config::MetadataGenConfigFile;
pub use index::{IndexFile, IndexFileEntry};
pub use provider::MetadataProvider;
pub use source::{MetadataSource, PackageDetails};
