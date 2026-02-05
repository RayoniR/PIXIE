// pixie/src/core/mod.rs
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeAlgorithm {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos3,
}

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub quality: u8,
    pub keep_aspect: bool,
    pub strip_metadata: bool,
    pub algorithm: ResizeAlgorithm,
    pub max_file_size: Option<u64>,
    pub format: Option<OutputFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Jpeg,
    Png,
    WebP,
    SameAsInput,
}

#[derive(Debug, Clone)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub has_exif: bool,
    pub file_size: u64,
}

#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub processed_count: usize,
    pub total_size_before: u64,
    pub total_size_after: u64,
    pub errors: Vec<(String, String)>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            scale: 0.0,
            quality: 85,
            keep_aspect: true,
            strip_metadata: false,
            algorithm: ResizeAlgorithm::Lanczos3,
            max_file_size: None,
            format: None,
        }
    }
}

impl ProcessConfig {
    pub fn validate(&self) -> Result<()> {
        if self.scale > 0.0 && (self.width > 0 || self.height > 0) {
            return Err(ImageToolError::InvalidParameter(
                "Cannot specify both scale and width/height".to_string(),
            ));
        }

        if self.width > 100_000 || self.height > 100_000 {
            return Err(ImageToolError::InvalidParameter(
                "Dimensions too large (max 100,000 pixels)".to_string(),
            ));
        }

        if self.quality == 0 || self.quality > 100 {
            return Err(ImageToolError::InvalidParameter(
                "Quality must be between 1 and 100".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ImageToolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Memory limit exceeded: {0}")]
    MemoryLimitExceeded(String),
}

pub type Result<T> = std::result::Result<T, ImageToolError>;

pub fn validate_config(config: &ProcessConfig) -> Result<()> {
    config.validate()
}