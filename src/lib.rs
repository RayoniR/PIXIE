mod cli;
mod core;
mod processors;
mod utils;

pub use cli::{Algorithm, Cli, Commands};
pub use core::{
    ImageProcessor, ImageToolError, ProcessConfig, ResizeAlgorithm, Result, 
    ImageMetadata, ProcessingStats, validate_config, OutputFormat
};
pub use processors::{
    BatchProcessor, Compressor, Loader, MetadataProcessor, Resizer
};
pub use utils::{
    calculate_aspect_ratio, format_file_size, generate_output_path,
    get_image_info, is_supported_format, validate_dimensions
};

pub mod prelude {
    pub use crate::{
        ImageProcessor, ProcessConfig, ResizeAlgorithm,
        BatchProcessor, Compressor, Loader, MetadataProcessor, Resizer
    };
}

// Re-export commonly used types
pub use image::DynamicImage;