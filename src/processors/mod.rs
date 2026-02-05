// pixie/src/processors/mod.rs
mod compressor;
mod loader;
mod metadata;
mod resizer;
mod batch;

pub use compressor::Compressor;
pub use loader::Loader;
pub use metadata::MetadataProcessor;
pub use resizer::{Resizer, ResizeMode};
pub use batch::BatchProcessor;

pub mod prelude {
    pub use super::{Compressor, Loader, MetadataProcessor, Resizer, BatchProcessor};
}